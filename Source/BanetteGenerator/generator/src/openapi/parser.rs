/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

/// Parses a string containing header include directives into a Vec<String>.
///
/// Supports two formats:
/// 1. Full format: `#include "a.h";#include "b.h";` or `#include <vector>;`
/// 2. Simplified format: `a.h;b.h` (will be converted to `#include "a.h"` format)
///
/// # Arguments
/// * `input` - A string that may contain multiple header includes in either format.
///
/// # Returns
/// A `Vec<String>` where each element is a complete `#include` directive.
pub fn parse_include_headers(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    // Check if input contains #include directive (full format)
    if input.contains("#include") {
        // Full format: split on #include and reconstruct
        input
            .split("#include")
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    // Reconstruct the include directive
                    let mut header = format!("#include {}", trimmed);
                    // Remove trailing semicolon for consistent output
                    if header.ends_with(';') {
                        header.pop();
                    }
                    Some(header)
                }
            })
            .collect()
    } else {
        // Simplified format: a.h;b.h -> #include "a.h", #include "b.h"
        input
            .split(';')
            .filter_map(|part| {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    // Wrap in #include "..." format
                    Some(format!("#include \"{}\"", trimmed))
                }
            })
            .collect()
    }
}
