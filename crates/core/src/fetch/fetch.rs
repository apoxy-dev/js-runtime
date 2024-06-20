use std::collections::HashMap;

use extism_pdk::*;
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct FetchRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct FetchResponse {
    status: u16,
    headers: HashMap<String, String>,
    body_offset: u64,
    error: Option<String>,
}

pub struct HttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Memory,
}

impl HttpResponse {
    pub fn status_code(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn body(&self) -> Vec<u8> {
        self.body.to_vec()
    }
}

#[link(wasm_import_module = "extism:host/user")]
extern "C" {
    fn _apoxy_fetch(req: u64, body: u64) -> u64;
}

pub fn request<T: ToMemory>(req: &FetchRequest, body: Option<T>) -> Result<HttpResponse, Error> {
    let mut fetch_msg = Vec::new();
    req.serialize(&mut Serializer::new(&mut fetch_msg))?;
    let fetch_mem = Memory::from_bytes(&fetch_msg)?;

    let body = match body {
        Some(b) => Some(b.to_memory()?),
        None => None,
    };
    let data = body.as_ref().map(|x| x.offset()).unwrap_or(0);

    let offs = unsafe { _apoxy_fetch(fetch_mem.offset(), data) };
    debug!("fetch response offset: {}", offs);
    match offs {
        o if o <= 0 => return Err(Error::msg("fetch failed")),
        _ => (),
    }
    let len = unsafe { extism::length_unsafe(offs) };
    let resp_mem = Memory(MemoryHandle {
        offset: offs,
        length: len,
    });
    let resp_bytes = resp_mem.to_vec();
    let mut deserialize = Deserializer::from_read_ref(&resp_bytes);
    let resp = FetchResponse::deserialize(&mut deserialize)?;
    let body_len = match resp.body_offset {
        0 => 0,
        _ => unsafe { extism::length_unsafe(resp.body_offset) },
    };

    debug!("response: {:?}", resp);

    match resp.error {
        Some(e) => return Err(Error::msg(e)),
        None => Ok(HttpResponse {
            status: resp.status,
            headers: resp.headers,
            body: Memory(MemoryHandle {
                offset: resp.body_offset,
                length: body_len,
            }),
        }),
    }
}
