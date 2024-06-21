declare global {
  /**
   * @internal
   */
  function __apoxy_req_body(abiReq: RequestABI): {
    error: boolean;
    message: string;
    bytes: Uint8Array;
  };
  /**
   * @internal
   */
  function __apoxy_req_send(
    obj: RequestImpl,
    abiReq: RequestABI,
    body: ArrayBuffer,
  ): { error: boolean; message: string };
  /**
   * @internal
   */
  function __apoxy_resp_body(abiRes: ResponseABI): {
    error: boolean;
    message: string;
    bytes: Uint8Array;
  };
  /**
   * @internal
   */
  function __apoxy_resp_send(
    abiRes: ResponseABI,
    body: Uint8Array,
  ): {
    error: boolean;
    message: string;
  };
  /**
   * @internal
   */
  function __apoxy_send_downstream(
    abiRes: ResponseABI,
    body: ArrayBuffer,
  ): {
    error: boolean;
    message: string;
  };

  interface Headers {
    append(name: string, value: string): void;

    delete(name: string): void;

    get(name: string): string | null;

    has(name: string): boolean;

    set(name: string, value: string): void;

    toObject(): Record<string, string>;
  }

  interface Request {
    method?:
      | "GET"
      | "HEAD"
      | "POST"
      | "PUT"
      | "DELETE"
      | "CONNECT"
      | "OPTIONS"
      | "TRACE"
      | "PATCH";

    url: string;

    proto: string;

    headers: Headers;

    content_len: number;

    host: string;

    remote_addr: string;

    body(): Uint8Array;

    set_body(body: Uint8Array): void;

    next(): Response;

    response(): Response | null;
  }

  interface Response {
    code: number;
    content_len: number;

    status(code: number): Response;

    headers(): Headers;

    body(): Uint8Array;

    send(body: Uint8Array): void;
  }

  type ServeHandler = (req: Request, res: Response) => void;

  var Env: {
    get(key: string): string | null;
  };

  var Apoxy: {
    env: typeof Env;
    serve(handler: ServeHandler): void;
  };

  /**
   * @internal
   */
  var __handler: (req: RequestABI) => void;
}

Apoxy.serve = new Proxy(Apoxy.serve, {
  apply(target, thisArg, [handler]) {
    __handler = (reqABI: RequestABI) => {
      try {
        let req = new RequestImpl(reqABI);
        let resp = new FilterResponseImpl();

        Promise.resolve(handler!(req, resp))
          .then(() => {
            if (resp.sendDownstream()) {
              console.debug("Sent response downstream");
              return;
            }
            let backend_resp = req.response() as BackendResponseImpl;
            if (backend_resp) {
              backend_resp.sendDownstream();
            }
          })
          .catch((e) => {
            console.error("[apoxy/js] Exception in handler:", e);
          });
      } catch (e) {
        console.error("[apoxy/js] Exception in handler:", e);
      }
    };
    return Reflect.apply(target, thisArg, [handler]);
  },
});

interface RequestABI {
  method: string;
  url: string;
  proto: string;
  proto_major: number;
  proto_minor: number;
  header: Record<string, string>;
  host: string;
  remote_addr: string;
  content_len: number;
}

class HeadersImpl implements Headers {
  private headers: Record<string, string> = {};

  constructor(headers: Record<string, string> = {}) {
    this.headers = headers;
  }

  append(name: string, value: string): void {
    this.headers = { ...this.headers, [name]: value };
  }

  delete(name: string): void {
    delete this.headers[name];
  }

  get(name: string): string | null {
    return this.headers[name] || null;
  }

  has(name: string): boolean {
    return this.headers[name] !== undefined;
  }

  set(name: string, value: string): void {
    this.headers = { ...this.headers, [name]: value };
  }

  toObject(): Record<string, string> {
    return this.headers;
  }
}

class RequestImpl implements Request {
  constructor(obj: RequestABI) {
    switch (obj.method) {
      case "GET":
      case "HEAD":
      case "POST":
      case "PUT":
      case "DELETE":
      case "CONNECT":
      case "OPTIONS":
      case "TRACE":
      case "PATCH":
        this.method = obj.method;
        break;
      case undefined:
        this.method = "GET";
        break;
      default:
        throw new Error(`Invalid method: "${obj.method}"`);
    }
    this.url = obj.url;
    this.proto = obj.proto;
    this.headers = new HeadersImpl(obj.header);
    this.host = obj.host;
    this.remote_addr = obj.remote_addr;
    this.content_len = obj.content_len;
  }

  method?:
    | "GET"
    | "HEAD"
    | "POST"
    | "PUT"
    | "DELETE"
    | "CONNECT"
    | "OPTIONS"
    | "TRACE"
    | "PATCH";

  url: string;

  proto: string;

  headers: Headers;

  content_len: number = 0;

  host: string;

  remote_addr: string;

  body(): Uint8Array {
    const result = __apoxy_req_body(this.abiReq());
    if (result.error === true) {
      throw new Error(result.message);
    }

    this.content_len = result.bytes.length;
    this._body = result.bytes;
    console.debug("Received request body from backend");
    return new Uint8Array(result.bytes);
  }

  set_body(body: Uint8Array): void {
    this._body = body;
    this.content_len = body.length;
    this._body_set = true;
  }

  next(): Response {
    console.debug("Sending request to backend");
    const result = __apoxy_req_send(
      this,
      this.abiReq(),
      this._body_set ? this._body.buffer : new ArrayBuffer(0),
    );
    if (result.error === true) {
      throw new Error(result.message);
    }

    console.debug("Received response from backend");
    this._response = new BackendResponseImpl(this._abi_response);
    return this._response;
  }

  response(): Response | null {
    return this._response;
  }

  private abiReq(): RequestABI {
    // Parse this.proto into major and minor
    let proto_major: number;
    let proto_minor: number;
    try {
      const [major, minor] = this.proto.split("/");
      proto_major = parseInt(major);
      proto_minor = parseInt(minor);
    } catch (e) {
      console.error("Failed to parse proto:", this.proto);
    }
    return {
      method: this.method!,
      url: this.url,
      proto: this.proto,
      proto_major: proto_major,
      proto_minor: proto_minor,
      header: this.headers.toObject(),
      host: this.host,
      remote_addr: this.remote_addr,
      content_len: this.content_len,
    };
  }
  private _body: Uint8Array | null = null;
  private _body_set: boolean = false;
  private _abi_response: ResponseABI | null = null;
  private _response: Response | null = null;
}

interface ResponseABI {
  status_code: number;
  content_len: number;
  header: Record<string, string>;
}

class FilterResponseImpl implements Response {
  code: number = 200;
  content_len: number = 0;

  constructor(json?: string) {
    if (!json) {
      return;
    }
    const obj: ResponseABI = JSON.parse(json);
    this.code = obj.status_code;
    this.content_len = obj.content_len;
    this._headers = new HeadersImpl(obj.header);
  }

  status(code: number): Response {
    this.code = code;
    this._set = true;
    return this;
  }

  headers(): Headers {
    return this._headers;
  }

  body(): Uint8Array {
    return this._body;
  }

  send(body: Uint8Array): void {
    console.debug("Setting response:", body.length);
    this.content_len = body.length;
    this._body = body;
    this._set = true;
  }

  sendDownstream(): boolean {
    console.debug("Response was altered:", this._set);
    if (!this._set) {
      return false;
    }

    console.debug("Sending downstream:", this._body ? this._body.length : 0);

    const abiResp: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };
    let body = this._body ? this._body : new Uint8Array(0);
    if (typeof body === "string") {
      body = new TextEncoder().encode(body);
    }
    const result = __apoxy_send_downstream(abiResp, body.buffer);
    if (result.error === true) {
      throw new Error(result.message);
    }
    return true;
  }

  private _headers: HeadersImpl = new HeadersImpl();
  private _body: Uint8Array | null = null;
  private _set: boolean = false;
}

class BackendResponseImpl implements Response {
  code: number = 200;
  content_len: number = 0;

  constructor(obj?: ResponseABI) {
    if (!obj) {
      return;
    }
    this.code = obj.status_code;
    this.content_len = obj.content_len;
    this._headers = new HeadersImpl(obj.header);
  }

  status(code: number): Response {
    this.code = code;
    return this;
  }

  headers(): Headers {
    return this._headers;
  }

  body(): Uint8Array {
    const abiResp: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };
    const result = __apoxy_resp_body(abiResp);
    if (result.error === true) {
      throw new Error(result.message);
    }

    this.content_len = result.bytes.length;
    this._body = result.bytes;
    return new Uint8Array(result.bytes);
  }

  send(body: Uint8Array): void {
    this.content_len = body.length;
    this._body = body;
  }

  sendDownstream(): void {
    const abiResp: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };
    // For the upstream case, we're modifying the response object in place
    // so we don't need to send the body to the VM.
    const result = __apoxy_resp_send(abiResp, this._body || new Uint8Array(0));
    if (result.error === true) {
      throw new Error(result.message);
    }
  }

  private _headers: Headers = new HeadersImpl();
  private _body: Uint8Array | null = null;
}

export {};
