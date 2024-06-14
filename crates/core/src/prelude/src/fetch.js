// Copyright 2023 VMware, Inc.
// SPDX-License-Identifier: Apache-2.0

import httpStatus from "http-status";

class Headers {
  constructor(initialHeaders) {
    let headers = {};

    // Initialize the headers
    for (const key in initialHeaders) {
      let value = initialHeaders[key];

      // Allow only string values
      if (typeof value === "string") {
        headers[key] = value;
      }
    }

    this.headers = headers;
  }

  append(key, value) {
    this.headers[key] = value;
    return value;
  }

  set(key, value) {
    this.append(key, value);
    return value;
  }

  delete(key) {
    let dropValue = delete this.headers[key];
    return dropValue;
  }

  get(key) {
    return this.headers[key];
  }

  toJSON() {
    return this.headers;
  }
}

// The response object to return the project response.
// It contains different helpers
class Response {
  constructor(body, options = {}) {
    // Process the body
    if (body instanceof String) {
      this.body = body.toString();
    } else {
      this.body = body;
    }

    if (options.headers instanceof Headers) {
      this.headers = options.headers;
    } else if (options.headers instanceof Object) {
      this.headers = new Headers(options.headers);
    } else {
      this.headers = new Headers({});
    }

    this.status = options.status || 200;
    this.statusText = options.statusText || httpStatus[this.status];
  }

  static redirect(url, status = 307) {
    return new Response(`Redirecting to ${url}`, {
      status,
      headers: {
        Location: url,
      },
    });
  }

  get ok() {
    return this.status >= 200 && this.status < 300;
  }

  defaultEncoding() {
    return "utf-8";
  }

  arrayBuffer() {
    let parsedBody = this.body;

    if (typeof this.body === "string") {
      try {
        // For now, we only consider the String|ArrayBuffer option
        parsedBody = new TextEncoder().encode(this.body);
      } catch (e) {
        return Promise.reject(
          `There was an error encoding the body: ${e}. Please, use the arrayBuffer() and TextDecoder method instead.`,
        );
      }
    }

    return parsedBody;
  }

  json() {
    let parsedBody = this.body;

    if (typeof this.body !== "string") {
      try {
        // For now, we only consider the String|ArrayBuffer option
        parsedBody = new TextDecoder(this.defaultEncoding()).decode(this.body);
      } catch (e) {
        return Promise.reject(
          `There was an error decoding the body: ${e}. Please, use the arrayBuffer() and TextDecoder method instead.`,
        );
      }
    }

    try {
      return Promise.resolve(JSON.parse(parsedBody));
    } catch (e) {
      return Promise.reject(`The body is not a valid JSON: ${e}`);
    }
  }

  text() {
    let parsedBody = this.body;

    if (typeof this.body !== "string") {
      try {
        // For now, we only consider the String|ArrayBuffer option
        parsedBody = new TextDecoder(this.defaultEncoding()).decode(this.body);
      } catch (e) {
        return Promise.reject(
          `There was an error decoding the body: ${e}. Please, use the arrayBuffer() and TextDecoder method instead.`,
        );
      }
    }

    return parsedBody;
  }

  toString() {
    return this.body;
  }
}

(function () {
  const __fetch = globalThis.__fetch;
  globalThis.fetch = (uri, opts) => {
    let optsWithDefault = {
      method: "GET",
      headers: {},
      body: null,
      ...opts,
    };

    if (
      optsWithDefault.body !== null &&
      typeof optsWithDefault.body !== "string"
    ) {
      try {
        optsWithDefault.body = new TextEncoder().encode(optsWithDefault.body);
      } catch (e) {
        return Promise.reject(
          `There was an error encoding the body: ${e}. Use a String or encode it using TextEncoder.`,
        );
      }
    }

    let result = __fetch(uri, optsWithDefault);

    if (result.error === true) {
      return Promise.reject(new Error(`[${result.type}] ${result.message}`));
    } else {
      let response = new Response(result.body, {
        headers: result.headers,
        status: result.status,
      });

      return Promise.resolve(response);
    }
  };

  Reflect.deleteProperty(globalThis, "__fetch");
})();

export { Headers, Response };
