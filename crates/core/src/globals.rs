use std::{borrow::Cow, collections::HashMap, str::from_utf8};

use anyhow::{anyhow, Context};
use chrono::{SecondsFormat, Utc};
use extism_pdk::*;
use javy::json;
use quickjs_wasm_rs::{JSContextRef, JSError, JSValue, JSValueRef};

static PRELUDE: &[u8] = include_bytes!("prelude/dist/index.js"); // if this panics, run `make` from the root

pub fn inject_globals(context: &JSContextRef) -> anyhow::Result<()> {
    let module = build_module_object(context)?;
    let console = build_console_object(context)?;
    let decoder = build_decoder(context)?;
    let encoder = build_encoder(context)?;
    let clock = build_clock(context)?;

    let apoxy = build_apoxy_object(context)?;
    let fetch = build_fetch_object(context)?;
    let apoxy_req_body = build_apoxy_req_body_object(context)?;
    let apoxy_req_send = build_apoxy_req_send_object(context)?;
    let apoxy_resp_body = build_apoxy_resp_body_object(context)?;
    let apoxy_resp_send = build_apoxy_resp_send_object(context)?;
    let apoxy_send_downstream = build_apoxy_send_downstream_object(context)?;

    let global = context.global_object()?;
    global.set_property("console", console)?;
    global.set_property("module", module)?;
    global.set_property("__decodeUtf8BufferToString", decoder)?;
    global.set_property("__encodeStringToUtf8Buffer", encoder)?;
    global.set_property("__getTime", clock)?;

    global.set_property("Apoxy", apoxy)?;
    global.set_property("__fetch", fetch)?;
    global.set_property("__apoxy_req_body", apoxy_req_body)?;
    global.set_property("__apoxy_req_send", apoxy_req_send)?;
    global.set_property("__apoxy_resp_body", apoxy_resp_body)?;
    global.set_property("__apoxy_resp_send", apoxy_resp_send)?;
    global.set_property("__apoxy_send_downstream", apoxy_send_downstream)?;

    context.eval_global(
        "script.js",
        "globalThis.module = {}; globalThis.module.exports = {}",
    )?;
    // need a *global* var for polyfills to work
    context.eval_global("script.js", "global = globalThis")?;
    context.eval_global("script.js", from_utf8(PRELUDE)?)?;

    Ok(())
}

fn get_args_as_str(args: &[JSValueRef]) -> anyhow::Result<String> {
    args.iter()
        .map(|arg| arg.as_str())
        .collect::<Result<Vec<&str>, _>>()
        .map(|vec| vec.join(" "))
        .context("Failed to convert args to string")
}

fn build_apoxy_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_object = context.object_value()?;
    let apoxy_serve = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, _args: &[JSValueRef]| Ok(JSValue::Undefined),
    )?;

    apoxy_object.set_property("serve", apoxy_serve)?;

    Ok(apoxy_object)
}

#[link(wasm_import_module = "extism:host/user")]
extern "C" {
    pub fn _apoxy_req_body(offs: u64) -> u64;
    pub fn _apoxy_req_send(req_offs: u64, body_offs: u64) -> u64;
    pub fn _apoxy_resp_body(offs: u64) -> u64;
    pub fn _apoxy_resp_send(resp_offs: u64, body_offs: u64) -> u64;
    pub fn _apoxy_send_downstream(resp_offs: u64, body_offs: u64) -> u64;
}

fn build_apoxy_req_body_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_req_body = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let req_bytes = json::transcode_output(*(args.first().unwrap()))?;
            let mem = Memory::from_bytes(req_bytes)?;

            let offs = unsafe { _apoxy_req_body(mem.offset()) };
            let len = unsafe { extism::length_unsafe(offs) };
            let mem = Memory(MemoryHandle {
                offset: offs,
                length: len,
            });

            let result = HashMap::from([("bytes", JSValue::ArrayBuffer(mem.to_vec()))]);
            Ok(JSValue::from_hashmap(result))
        },
    )?;

    Ok(apoxy_req_body)
}

fn build_apoxy_req_send_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_req_send = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let this = args.first().unwrap();

            let req_bytes = json::transcode_output(*(args.get(1).unwrap()))?;
            let req_mem = Memory::from_bytes(req_bytes)?;

            let body_bytes = args.get(2).unwrap().as_bytes()?;
            let body_mem = Memory::from_bytes(body_bytes)?;

            let offs = unsafe { _apoxy_req_send(req_mem.offset(), body_mem.offset()) };
            let len = unsafe { extism::length_unsafe(offs) };
            let mem = Memory(MemoryHandle {
                offset: offs,
                length: len,
            });

            let response = json::transcode_input(_ctx, mem.to_vec().as_slice()).unwrap();
            this.set_property("_abi_response", response).unwrap();

            Ok(JSValue::from_hashmap(HashMap::from([(
                "error",
                JSValue::Bool(false),
            )])))
        },
    )?;

    Ok(apoxy_req_send)
}

fn build_apoxy_resp_body_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_resp_body = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let resp_bytes = json::transcode_output(*(args.first().unwrap()))?;
            let mem = Memory::from_bytes(resp_bytes)?;

            let offs = unsafe { _apoxy_resp_body(mem.offset()) };
            let len = unsafe { extism::length_unsafe(offs) };
            let mem = Memory(MemoryHandle {
                offset: offs,
                length: len,
            });

            let result = HashMap::from([("bytes", JSValue::ArrayBuffer(mem.to_vec()))]);
            Ok(JSValue::from_hashmap(result))
        },
    )?;

    Ok(apoxy_resp_body)
}

fn build_apoxy_resp_send_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_resp_send = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let resp_bytes = json::transcode_output(*(args.first().unwrap()))?;
            let resp_mem = Memory::from_bytes(resp_bytes)?;

            let body_bytes = args.get(1).unwrap().as_bytes()?;
            let body_mem = Memory::from_bytes(body_bytes)?;

            let ret = unsafe { _apoxy_resp_send(resp_mem.offset(), body_mem.offset()) };
            if ret != 0 {
                return Ok(JSValue::from_hashmap(HashMap::from([
                    ("error", JSValue::Bool(true)),
                    (
                        "message",
                        JSValue::String("Failed to send response".to_string()),
                    ),
                ])));
            }

            Ok(JSValue::from_hashmap(HashMap::from([(
                "error",
                JSValue::Bool(false),
            )])))
        },
    )?;

    Ok(apoxy_resp_send)
}

fn build_apoxy_send_downstream_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let apoxy_send_downstream = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let resp_bytes = json::transcode_output(*(args.first().unwrap()))?;
            let resp_mem = Memory::from_bytes(resp_bytes)?;

            let body_bytes = args.get(1).unwrap().as_bytes()?;
            let body_mem = Memory::from_bytes(body_bytes)?;

            debug!(
                "Sending downstream response with {} bytes",
                body_bytes.len()
            );
            let ret = unsafe { _apoxy_send_downstream(resp_mem.offset(), body_mem.offset()) };
            if ret != 0 {
                return Ok(JSValue::from_hashmap(HashMap::from([
                    ("error", JSValue::Bool(true)),
                    (
                        "message",
                        JSValue::String("Failed to send response".to_string()),
                    ),
                ])));
            }

            Ok(JSValue::from_hashmap(HashMap::from([(
                "error",
                JSValue::Bool(false),
            )])))
        },
    )?;

    Ok(apoxy_send_downstream)
}

fn build_console_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let console_debug_callback = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let stmt = get_args_as_str(args)?;
            debug!("{}", stmt);
            Ok(JSValue::Undefined)
        },
    )?;
    let console_info_callback = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let stmt = get_args_as_str(args)?;
            info!("{}", stmt);
            Ok(JSValue::Undefined)
        },
    )?;
    let console_warn_callback = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let stmt = get_args_as_str(args)?;
            warn!("{}", stmt);
            Ok(JSValue::Undefined)
        },
    )?;
    let console_error_callback = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let stmt = get_args_as_str(args)?;
            error!("{}", stmt);
            Ok(JSValue::Undefined)
        },
    )?;

    let console_object = context.object_value()?;

    // alias for console.info
    console_object.set_property("log", console_info_callback)?;

    console_object.set_property("debug", console_debug_callback)?;
    console_object.set_property("info", console_info_callback)?;
    console_object.set_property("warn", console_warn_callback)?;
    console_object.set_property("error", console_error_callback)?;

    Ok(console_object)
}

fn build_module_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let exports = context.object_value()?;
    let module_obj = context.object_value()?;
    module_obj.set_property("exports", exports)?;
    Ok(module_obj)
}

fn build_fetch_object(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    let fetch_callback = context.wrap_callback(
        |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
            let url = args.get(0).unwrap().as_str()?;
            let opts: HashMap<String, JSValue> = args.get(1).unwrap().try_into()?;

            let method = opts.get("method").unwrap().to_string();
            match method.as_str() {
                "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" => {}
                _ => return Err(anyhow!("Invalid method: {}", method)),
            }
            let mut http_req = HttpRequest::new(url).with_method(method.to_string());

            let headers = opts.get("headers").unwrap();
            if let JSValue::Object(headers) = headers {
                for (key, value) in headers {
                    http_req = http_req.with_header(key, value.to_string());
                }
            }

            let body = opts.get("body").unwrap_or(&JSValue::Undefined);
            let mut http_body: Option<String> = None;
            if let JSValue::String(body) = body {
                http_body = Some(body.clone());
            }

            match http::request::<String>(&http_req, http_body) {
                Ok(resp) => {
                    let parsed_result = HashMap::from([
                        ("status", JSValue::Int(i32::from(resp.status_code()))),
                        ("body", JSValue::ArrayBuffer(resp.body())),
                    ]);
                    Ok(JSValue::from_hashmap(parsed_result))
                }
                Err(e) => Ok(JSValue::from_hashmap(HashMap::from([
                    ("error", JSValue::Bool(true)),
                    ("type", JSValue::String("InternalError".to_string())),
                    ("message", JSValue::String(e.to_string())),
                ]))),
            }
        },
    )?;
    Ok(fetch_callback)
}

fn build_clock(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    context.wrap_callback(get_time())
}

fn build_decoder(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    context.wrap_callback(decode_utf8_buffer_to_js_string())
}

fn build_encoder(context: &JSContextRef) -> anyhow::Result<JSValueRef> {
    context.wrap_callback(encode_js_string_to_utf8_buffer())
}

fn get_time() -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx: &JSContextRef, _this: JSValueRef, _args: &[JSValueRef]| {
        let now = Utc::now();
        // This format is compatible with JavaScript's Date constructor
        let formatted = now.to_rfc3339_opts(SecondsFormat::Millis, true);
        Ok(formatted.into())
    }
}

fn decode_utf8_buffer_to_js_string(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
        if args.len() != 5 {
            return Err(anyhow!("Expecting 5 arguments, received {}", args.len()));
        }

        let buffer: Vec<u8> = args[0].try_into()?;
        let byte_offset: usize = args[1].try_into()?;
        let byte_length: usize = args[2].try_into()?;
        let fatal: bool = args[3].try_into()?;
        let ignore_bom: bool = args[4].try_into()?;

        let mut view = buffer
            .get(byte_offset..(byte_offset + byte_length))
            .ok_or_else(|| {
                anyhow!("Provided offset and length is not valid for provided buffer")
            })?;

        if !ignore_bom {
            view = match view {
                // [0xEF, 0xBB, 0xBF] is the UTF-8 BOM which we want to strip
                [0xEF, 0xBB, 0xBF, rest @ ..] => rest,
                _ => view,
            };
        }

        let str =
            if fatal {
                Cow::from(from_utf8(view).map_err(|_| {
                    JSError::Type("The encoded data was not valid utf-8".to_string())
                })?)
            } else {
                String::from_utf8_lossy(view)
            };
        Ok(str.to_string().into())
    }
}

fn encode_js_string_to_utf8_buffer(
) -> impl FnMut(&JSContextRef, JSValueRef, &[JSValueRef]) -> anyhow::Result<JSValue> {
    move |_ctx: &JSContextRef, _this: JSValueRef, args: &[JSValueRef]| {
        if args.len() != 1 {
            return Err(anyhow!("Expecting 1 argument, got {}", args.len()));
        }

        let js_string: String = args[0].try_into()?;
        Ok(js_string.into_bytes().into())
    }
}
