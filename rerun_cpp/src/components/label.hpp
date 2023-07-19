// NOTE: This file was autogenerated by re_types_builder; DO NOT EDIT.
// Based on "crates/re_types/definitions/rerun/components/label.fbs"

#pragma once

#include <cstdint>
#include <string>
#include <utility>

namespace rr {
    namespace components {
        /// A String label component.
        struct Label {
            std::string value;

            Label(std::string value) : value(std::move(value)) {}
        };
    } // namespace components
} // namespace rr