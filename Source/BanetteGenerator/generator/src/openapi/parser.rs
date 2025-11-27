/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

/// Parses a string containing multiple `#include` directives to a Vec<String>.
///
/// # Arguments
/// * `input` - A string that may contain multiple `#include` directives concatenated together,
///   e.g., `#include "a.h";#include "b.h";`.
///
/// # Returns
/// A `Vec<String>` where each element is a complete `#include` directive.
pub fn parse_include_headers(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    input
        .split("#include")
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                None
            } else {
                // Reconstruct the include directive
                let mut header = format!("#include {}", trimmed);
                // Ensure it ends with a semicolon if not already
                if !header.ends_with(';') {
                    header.push(';');
                }
                Some(header)
            }
        })
        .collect()
}
