#pragma once
#include "gguf.h"
#include "e5_normalizer.hpp"
#include <cstdio>
#include <memory>
#include <vector>

struct FileCloser {
    void operator()(FILE *file) const noexcept { if (file) fclose(file); }
};
using PrivateModelFile = std::unique_ptr<FILE, FileCloser>;

// The original pinned Q8_0 tensors remain byte-identical. Only legacy metadata
// is repaired to the current official Unigram tokenizer conventions.
inline FILE *prepare_e5(const char *path) {
    auto source = PrivateModelFile(fopen(path, "rb"));
    if (!source) return nullptr;
    auto metadata = std::unique_ptr<gguf_context, decltype(&gguf_free)>(gguf_init_from_file_ptr(source.get(), {true, nullptr}), gguf_free);
    if (!metadata) return nullptr;
    const auto offset = gguf_get_data_offset(metadata.get());
    gguf_set_val_u32(metadata.get(), "tokenizer.ggml.token_type_count", 2);
    gguf_set_val_str(metadata.get(), "tokenizer.ggml.model", "t5");
    gguf_set_val_str(metadata.get(), "tokenizer.ggml.pre", "default");
    gguf_set_val_bool(metadata.get(), "tokenizer.ggml.add_space_prefix", true);
    gguf_set_val_bool(metadata.get(), "tokenizer.ggml.remove_extra_whitespaces", true);
    gguf_set_arr_data(metadata.get(), "tokenizer.ggml.precompiled_charsmap", GGUF_TYPE_UINT8, e5_normalizer, sizeof(e5_normalizer));
    FILE *output = tmpfile();
    if (!output) return nullptr;
    if (!gguf_write_to_file_ptr(metadata.get(), output, true) || fseek(source.get(), offset, SEEK_SET) != 0) { fclose(output); return nullptr; }
    std::vector<unsigned char> buffer(65536);
    size_t count;
    while ((count = fread(buffer.data(), 1, buffer.size(), source.get())) > 0) {
        if (fwrite(buffer.data(), 1, count, output) != count) { fclose(output); return nullptr; }
    }
    if (ferror(source.get()) || fflush(output) != 0 || fseek(output, 0, SEEK_SET) != 0) { fclose(output); return nullptr; }
    return output;
}
