// Copyright 2024, The Extism Authors.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
// WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
// ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
// (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
// LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
// ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use extism_pdk::*;
use once_cell::sync::OnceCell;
use quickjs_wasm_rs::{JSContextRef, JSValue, JSValueRef};
use std::io;
use std::io::Read;

mod fetch;
mod globals;

static mut CONTEXT: OnceCell<JSContextRef> = OnceCell::new();
static mut USER_CODE: OnceCell<String> = OnceCell::new();

#[export_name = "wizer.initialize"]
extern "C" fn init() {
    let context = JSContextRef::default();
    globals::inject_globals(&context).expect("Failed to initialize globals");

    let mut code = String::new();
    io::stdin().read_to_string(&mut code).unwrap();
    unsafe { USER_CODE.set(code).unwrap() };

    unsafe {
        CONTEXT.set(context).unwrap();
    }
}

fn js_context<'a>() -> &'a JSContextRef {
    unsafe {
        if CONTEXT.get().is_none() {
            init()
        }

        let context = CONTEXT.get_unchecked();
        context
    }
}

fn code() -> &'static str {
    unsafe { USER_CODE.get_unchecked() }
}

fn convert_js_value<'a>(context: &'a JSContextRef, v: &JSValue) -> JSValueRef<'a> {
    match v {
        JSValue::Undefined => context.undefined_value().unwrap(),
        JSValue::Null => context.null_value().unwrap(),
        JSValue::Bool(b) => context.value_from_bool(*b).unwrap(),
        JSValue::Int(i) => context.value_from_i32(*i).unwrap(),
        JSValue::Float(f) => context.value_from_f64(*f).unwrap(),
        JSValue::String(s) => context.value_from_str(s.as_str()).unwrap(),
        JSValue::Array(a) => {
            let arr = context.array_value().unwrap();
            for x in a.iter() {
                arr.append_property(convert_js_value(context, x)).unwrap();
            }
            arr
        }
        JSValue::ArrayBuffer(buf) => context.array_buffer_value(buf.as_slice()).unwrap(),
        JSValue::Object(x) => {
            let obj = context.object_value().unwrap();
            for (k, v) in x.iter() {
                obj.set_property(k.as_str(), convert_js_value(context, v))
                    .unwrap();
            }
            obj
        }
    }
}

fn export_names(exports: JSValueRef<'static>) -> anyhow::Result<Vec<String>> {
    let mut properties = exports.properties()?;
    let mut key = properties.next_key()?;
    let mut keys: Vec<String> = vec![];
    while key.is_some() {
        keys.push(key.unwrap().as_str()?.to_string());
        key = properties.next_key()?;
    }
    keys.sort();
    Ok(keys)
}

#[plugin_fn]
pub fn _start() -> FnResult<()> {
    let context = js_context();
    let code = code();

    context.eval_global("script.js", code)?;

    Ok(())
}

#[plugin_fn]
pub fn _apoxy_start() -> FnResult<()> {
    let context = js_context();

    let req = javy::json::transcode_input(&context, input_bytes().as_slice())?;
    context.global_object()?.set_property(
        "__backend_mode",
        req.get_property("backend_mode")?,
    )?;
    context
        .global_object()?
        .get_property("__handler")?
        .call(&context.undefined_value().unwrap(), &[req])?;

    // Execute all pending operations (e.g promises).
    while context.is_pending() {
        context.execute_pending()?;
    }

    Ok(())
}
