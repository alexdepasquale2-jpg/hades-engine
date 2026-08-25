//! Typed client SDK for The Big Computer engine (M17).
//!
//! - [`TbcHttpClient`] — REST API against `tbc-server`
//! - Wire protocol types re-exported from `tbc_engine::transport` for QUIC gateway clients

pub mod error;
pub mod http;
pub mod types;

pub use error::SdkError;
pub use http::TbcHttpClient;
pub use types::{ConsentResponse, LoginResponse, StatusResponse};

pub use tbc_engine::wire_json::id_json;
pub use tbc_engine::transport::{
    decode_move_datagram, decode_reliable, drain_reliable, encode_move_datagram, encode_reliable,
    WireMessage, WIRE_MAGIC,
};

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn assist_wire_roundtrip() {
        let msg = WireMessage::assist(42, Some(7));
        let dec = decode_reliable(&encode_reliable(&msg).unwrap()).unwrap();
        assert_eq!(dec.kind, "assist");
        assert_eq!(dec.fwau_u128(), Some(42));
    }
}
