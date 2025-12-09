/*
 * Copyright 2019-Present tarnishablec. All Rights Reserved.
 */

pub mod http_request_builder;
pub mod is_required;
pub mod path_to_func_name;
pub mod request_body_schema;
pub mod response_body_schema;
pub mod tags_to_pipe_separated;
pub mod to_ue_type;

use tera::Tera;

pub fn register_all_filters(tera: &mut Tera) {
    tera.register_filter("to_ue_type", to_ue_type::to_ue_type_filter);
    tera.register_filter("is_required", is_required::is_required_filter);
    tera.register_filter(
        "tags_to_pipe_separated",
        tags_to_pipe_separated::tags_to_pipe_separated_filter,
    );
    tera.register_filter(
        "request_body_schema",
        request_body_schema::request_body_schema_filter,
    );
    tera.register_filter(
        "response_body_schema",
        response_body_schema::response_body_schema_filter,
    );
    tera.register_filter(
        "path_to_func_name",
        path_to_func_name::path_to_func_name_filter,
    );
    tera.register_filter(
        "http_request_builder",
        http_request_builder::http_request_builder_filter,
    );
}

#[cfg(test)]
pub mod tests {
    use serde_json::{to_value, Value};
    use std::collections::HashMap;

    pub fn create_method_args(method: &str) -> HashMap<String, Value> {
        let mut args = HashMap::new();
        args.insert("method".to_string(), to_value(method).unwrap());
        args
    }
}
