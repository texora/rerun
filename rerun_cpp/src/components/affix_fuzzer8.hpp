// NOTE: This file was autogenerated by re_types_builder; DO NOT EDIT.
// Based on "crates/re_types/definitions/rerun/testing/components/fuzzy.fbs"

#pragma once

#include <cstdint>
#include <optional>
#include <utility>

namespace rr {
    namespace components {
        struct AffixFuzzer8 {
            std::optional<float> single_float_optional;

            AffixFuzzer8(std::optional<float> single_float_optional)
                : single_float_optional(std::move(single_float_optional)) {}
        };
    } // namespace components
} // namespace rr