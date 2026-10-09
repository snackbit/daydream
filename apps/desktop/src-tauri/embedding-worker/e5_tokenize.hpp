#pragma once
#include "llama.h"
#include <array>
#include <cstring>
#include <stdexcept>
#include <string>
#include <vector>

// HF's AddedToken boundary preserves a trailing normalized whitespace piece.
// llama.cpp's Unigram normalizer trims it, so tokenize normal spans separately
// and restore that piece. The sentinel probe uses the same official charsmap;
// it also distinguishes whitespace-only spans from removed zero-width text.
inline std::vector<llama_token> e5_tokenize(const llama_vocab *vocab, const std::string &text) {
    if (std::strcmp(llama_vocab_get_text(vocab, 6), "\xe2\x96\x81") != 0) throw std::runtime_error("model_incompatible");
    std::vector<llama_token> result = {0};
    auto normal_span = [&](const std::string &span) {
        if (span.empty()) return;
        std::vector<llama_token> tokens(528);
        int count = llama_tokenize(vocab, span.data(), span.size(), tokens.data(), tokens.size(), false, false);
        if (count < 0) throw std::runtime_error("text_token_limit");
        result.insert(result.end(), tokens.begin(), tokens.begin() + count);
        const std::string probe = "a" + span + "a";
        count = llama_tokenize(vocab, probe.data(), probe.size(), tokens.data(), tokens.size(), false, false);
        if (count < 0) throw std::runtime_error("text_token_limit");
        std::string decoded;
        for (int i = 0; i < count; ++i) {
            char piece[256];
            int size = llama_token_to_piece(vocab, tokens[i], piece, sizeof(piece), 0, true);
            if (size < 0) throw std::runtime_error("tokenizer_failed");
            decoded.append(piece, size);
        }
        if (decoded.size() >= 2 && decoded[decoded.size()-2] == ' ' && decoded.back() == 'a') result.push_back(6);
    };
    const std::array<std::pair<const char *, llama_token>, 5> special = {{{"<s>",0},{"<pad>",1},{"</s>",2},{"<unk>",3},{"<mask>",250001}}};
    size_t position = 0;
    while (position < text.size()) {
        size_t next = std::string::npos;
        const char *piece = nullptr;
        llama_token token = 0;
        for (const auto &candidate : special) {
            const auto found = text.find(candidate.first, position);
            if (found < next) {next = found; piece = candidate.first; token = candidate.second;}
        }
        if (!piece) {normal_span(text.substr(position)); break;}
        normal_span(text.substr(position, next-position));
        result.push_back(token);
        position = next + std::strlen(piece);
    }
    result.push_back(2);
    if (result.size() > 512) throw std::runtime_error("text_token_limit");
    return result;
}
