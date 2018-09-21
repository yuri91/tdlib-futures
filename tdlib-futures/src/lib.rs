extern crate tdjson_sys;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate futures;
extern crate serde;
extern crate tokio_core;
extern crate serde_aux;

mod tdjson;
pub use tdjson::set_log_file;
pub use tdjson::set_log_verbosity_level;

pub mod client;
pub mod types;
