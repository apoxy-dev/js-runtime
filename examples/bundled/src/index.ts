export function greet() {
  Var.set("name", "MAYBESteve");
  let extra = new TextEncoder().encode("aaa");
  let decoded = new TextDecoder().decode(extra);

  console.log(typeof fetch);
  fetch("https://jsonplaceholder.typicode.com/todos/1")
    .then((r) => {
      console.log("fetch", r.status);

      const name = Var.getString("name") || "unknown";
      const apiKey = Config.get("SOME_API_KEY") || "unknown";

      dec = new TextDecoder();
      Host.outputString(
        `Hello, ${Host.inputString()} (or is it ${name}???) ${decoded} ${new Date().toString()}\n\n${
           dec.decode(r.body)
        }\n\n ==== KEY: ${apiKey}`
      );
    });
}

// test this bundled.wasm like so:
// extism call ../bundled.wasm greet --input "steve" --wasi --allow-host "*" --config SOME_API_KEY=123456789
