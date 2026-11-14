pub mod local_server;

pub use local_server::{
    parse_hex_color, parse_http_request, preset_color_for_id, spawn_local_api_server,
    LocalApiCommand, LocalApiResponse, DEFAULT_LOCAL_API_ADDR,
};
