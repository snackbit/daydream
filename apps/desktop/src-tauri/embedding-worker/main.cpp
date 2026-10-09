#include "llama.h"
#include "ggml-backend.h"
#include "e5_compat.hpp"
#include "e5_tokenize.hpp"
#include <nlohmann/json.hpp>
#include <algorithm>
#include <cmath>
#include <cstring>
#include <iostream>
#include <memory>
#include <string>
#include <vector>
#ifdef __linux__
#include <signal.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <unistd.h>
#endif

using json = nlohmann::json;
constexpr size_t max_line = 300000;
constexpr int max_tokens = 512;

// Prompts and upstream diagnostic text must never reach a log or stderr.
static void quiet_log(ggml_log_level, const char *message, void *) {
#ifdef DAYDREAM_MODEL_VALIDATION
    std::cerr << message;
#else
    (void)message;
#endif
}
static bool read_request(std::string &line) {
    line.clear();
    char c;
    while (std::cin.get(c)) {
        if (c == '\n') return true;
        if (line.size() >= max_line) throw std::runtime_error("request_too_large");
        line.push_back(c);
    }
    return !line.empty();
}

int main(int argc, char **argv) {
    if (argc != 4) return 2;
#ifdef __linux__
    // The process cache lives for the app lifetime; do not orphan inference if
    // the parent exits or crashes while a request is running.
    if (prctl(PR_SET_PDEATHSIG, SIGKILL) != 0 || getppid() == 1) return 2;
    const rlimit no_core = {0, 0};
    if (setrlimit(RLIMIT_CORE, &no_core) != 0 || prctl(PR_SET_DUMPABLE, 0) != 0) return 2;
#endif
    llama_log_set(quiet_log, nullptr);
    ggml_log_set(quiet_log, nullptr);
    llama_backend_init();
    const std::string model_id = argv[2];
    const std::string backend = argv[3];
    if (model_id != "multilingual-e5-small-q8_0" && model_id != "bge-small-en-v1.5-q8_0") return 2;
    if (backend != "cpu" && backend != "auto" && backend != "vulkan") return 2;
    auto params = llama_model_default_params();
    ggml_backend_dev_t selected = nullptr;
    if (backend != "cpu") {
        for (size_t i = 0; i < ggml_backend_dev_count(); ++i) {
            auto dev = ggml_backend_dev_get(i);
            const auto reg = ggml_backend_dev_backend_reg(dev);
            const auto type = ggml_backend_dev_type(dev);
            if (std::string(ggml_backend_reg_name(reg)) == "Vulkan" &&
                (type == GGML_BACKEND_DEVICE_TYPE_GPU || type == GGML_BACKEND_DEVICE_TYPE_IGPU)) {
                selected = dev;
                break;
            }
        }
        if (backend == "vulkan" && !selected) {
            std::cout << json({{"error", "vulkan_unavailable"}}).dump() << std::endl;
            return 3;
        }
    }
    ggml_backend_dev_t devices[] = {selected, nullptr};
    params.devices = devices;
    params.n_gpu_layers = selected ? 999 : 0;
    auto prepared = PrivateModelFile(model_id == "multilingual-e5-small-q8_0" ? prepare_e5(argv[1]) : nullptr);
    if (model_id == "multilingual-e5-small-q8_0" && !prepared) return 4;
    auto load_model = [&]() {
        if (prepared) { rewind(prepared.get()); return llama_model_load_from_file_ptr(prepared.get(), params); }
        return llama_model_load_from_file(argv[1], params);
    };
    using Model = std::unique_ptr<llama_model, decltype(&llama_model_free)>;
    Model model(load_model(), llama_model_free);
    // Auto can fall back after a GPU allocation/load failure. Explicit Vulkan cannot.
    if (!model && backend == "auto" && selected) {
        selected = nullptr;
        devices[0] = nullptr;
        params.n_gpu_layers = 0;
        model.reset(load_model());
    }
    if (!model || llama_model_n_embd(model.get()) != 384) {
        std::cout << json({{"error", "model_incompatible"}}).dump() << std::endl;
        return 4;
    }
    auto ctx_params = llama_context_default_params();
    ctx_params.n_ctx = max_tokens;
    ctx_params.n_batch = max_tokens;
    ctx_params.n_ubatch = max_tokens;
    ctx_params.n_threads = 2;
    ctx_params.n_threads_batch = 2;
    ctx_params.embeddings = true;
    ctx_params.attention_type = LLAMA_ATTENTION_TYPE_NON_CAUSAL;
    ctx_params.pooling_type = model_id == "multilingual-e5-small-q8_0" ? LLAMA_POOLING_TYPE_MEAN : LLAMA_POOLING_TYPE_CLS;
    using Context = std::unique_ptr<llama_context, decltype(&llama_free)>;
    Context ctx(llama_init_from_model(model.get(), ctx_params), llama_free);
    if (!ctx && backend == "auto" && selected) {
        selected = nullptr;
        devices[0] = nullptr;
        params.n_gpu_layers = 0;
        model.reset(load_model());
        if (model) ctx.reset(llama_init_from_model(model.get(), ctx_params));
    }
    if (!ctx) return 4;
    const auto vocab = llama_model_get_vocab(model.get());
    std::cout << json({{"ready", true}, {"dimensions", 384}, {"backend", selected ? "vulkan" : "cpu"}}).dump() << std::endl;
    std::string line;
    try {
        while (read_request(line)) {
            try {
                auto request = json::parse(line);
                auto texts = request.at("texts").get<std::vector<std::string>>();
                auto purpose = request.at("purpose").get<std::string>();
                if (texts.empty() || texts.size() > 32 || (purpose != "query" && purpose != "document")) throw std::runtime_error("invalid_request");
                json vectors = json::array();
                json token_counts = json::array();
#ifdef DAYDREAM_MODEL_VALIDATION
                json token_ids = json::array();
#endif
                for (auto text : texts) {
                    if (text.empty() || text.size() > 8192) throw std::runtime_error("invalid_text_length");
                    if (model_id == "multilingual-e5-small-q8_0") text = (purpose == "query" ? "query: " : "passage: ") + text;
                    else if (purpose == "query") text = "Represent this sentence for searching relevant passages: " + text;
                    std::vector<llama_token> tokens(max_tokens);
                    int count;
                    if (model_id == "multilingual-e5-small-q8_0") {
                        tokens = e5_tokenize(vocab, text);
                        count = tokens.size();
                    } else {
                        count = llama_tokenize(vocab, text.data(), text.size(), tokens.data(), max_tokens, true, true);
                    }
                    if (count <= 0 || count > max_tokens) throw std::runtime_error("text_token_limit");
                    auto batch = llama_batch_init(count, 0, 1);
                    batch.n_tokens = count;
                    for (int i = 0; i < count; ++i) {
                        batch.token[i] = tokens[i];
                        batch.pos[i] = i;
                        batch.n_seq_id[i] = 1;
                        batch.seq_id[i][0] = 0;
                        batch.logits[i] = true;
                    }
                    llama_memory_clear(llama_get_memory(ctx.get()), true);
                    int status = llama_model_has_encoder(model.get()) ? llama_encode(ctx.get(), batch) : llama_decode(ctx.get(), batch);
                    llama_batch_free(batch);
                    if (status != 0) throw std::runtime_error("inference_failed");
                    const float *embedding = llama_get_embeddings_seq(ctx.get(), 0);
                    if (!embedding) throw std::runtime_error("pooling_failed");
                    std::vector<float> vector(embedding, embedding + 384);
                    double norm = 0;
                    for (auto value : vector) {
                        if (!std::isfinite(value)) throw std::runtime_error("nonfinite_embedding");
                        norm += double(value) * value;
                    }
                    if (norm <= 0) throw std::runtime_error("zero_embedding");
                    for (auto &value : vector) value /= std::sqrt(norm);
                    vectors.push_back(vector);
                    token_counts.push_back(count);
#ifdef DAYDREAM_MODEL_VALIDATION
                    token_ids.push_back(std::vector<llama_token>(tokens.begin(), tokens.begin() + count));
#endif
                }
                json response = {{"vectors", vectors}, {"tokenCounts", token_counts}};
#ifdef DAYDREAM_MODEL_VALIDATION
                response["tokenIds"] = token_ids;
#endif
                std::cout << response.dump() << std::endl;
            } catch (const std::runtime_error &error) {
                const std::string code = error.what();
                const std::vector<std::string> allowed = {"invalid_request", "invalid_text_length", "text_token_limit", "inference_failed", "pooling_failed", "nonfinite_embedding", "zero_embedding"};
                std::cout << json({{"error", std::find(allowed.begin(), allowed.end(), code) != allowed.end() ? code : "embedding_request_failed"}}).dump() << std::endl;
            } catch (...) {
                // Error codes are fixed; JSON/llama exception messages may contain text.
                std::cout << json({{"error", "embedding_request_failed"}}).dump() << std::endl;
            }
        }
    } catch (...) { return 5; }
    return 0;
}
