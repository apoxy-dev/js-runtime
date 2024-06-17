declare global {
  /**
   * @internal
   */
  function __apoxy_req_body(abiReq: RequestABI): {
    error: boolean;
    message: string;
    bytes: Uint8Array;
  };
  function __apoxy_req_send(
    obj: RequestImpl,
    abiReq: RequestABI,
    body: Uint8Array,
  ): { error: boolean; message: string };
  //function __apoxy_resp_body,
  //function __apoxy_resp_send,
  //function __apoxy_send_downstream,

  interface Headers {
    append(name: string, value: string): void;

    delete(name: string): void;

    get(name: string): string | null;

    has(name: string): boolean;

    set(name: string, value: string): void;

    toObject(): Record<string, string>;
  }

  interface Request {
    url: string;

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

    headers: Headers;

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

    body(): string;

    send(body: string): void;
  }

  type ServeHandler = (req: Request, res: Response) => void;

  var Apoxy: {
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

        handler!(req, resp);

        resp.sendDownstream();
        let backend_resp = req.response() as BackendResponseImpl;
        if (backend_resp) {
          backend_resp.sendDownstream();
        }
      } catch (e) {
        console.error(e);
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
    this.url = obj.url;
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
        throw new Error(`Invalid method: ${obj.method}`);
    }
    this.headers = new HeadersImpl(obj.header);
  }

  url: string;
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

  headers: Headers;

  content_len: number = 0;

  body(): Uint8Array {
    const result = __apoxy_req_body(this.abiReq());
    if (result.error === true) {
      throw new Error(result.message);
    }

    this.content_len = result.bytes.length;
    this._body = result.bytes;
    return new Uint8Array(result.bytes);
  }

  set_body(body: Uint8Array): void {
    this._body = body;
    this.content_len = body.length;
    this._body_set = true;
  }

  next(): Response {
    const result = __apoxy_req_send(
      this,
      this.abiReq(),
      this._body_set ? this._body : new Uint8Array(0),
    );
    if (result.error === true) {
      throw new Error(result.message);
    } else {
      this._response = new BackendResponseImpl(this._abi_response);
      return this._response;
    }
  }

  response(): Response | null {
    return this._response;
  }

  private abiReq(): RequestABI {
    return {
      method: this.method!,
      url: this.url,
      proto: "HTTP",
      proto_major: 1,
      proto_minor: 1,
      header: this.headers.toObject(),
      host: "localhost",
      remote_addr: "",
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

  body(): string {
    return this._body;
  }

  send(body: string): void {
    this.content_len = body.length;
    this._body = body;
    this._set = true;
  }

  sendDownstream(): void {
    if (!this._set) {
      return;
    }

    const resp_json: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };

    const resp_str = JSON.stringify(resp_json);
    console.log("Sending response downstream: ", resp_str);

    /*
    const ret_mem = _apoxy_send_downstream(
      Memory.fromString(resp_str).offset,
      Memory.fromString(this._body).offset,
    );
    console.log("Sent response downstream: ", ret_mem);
    if (ret_mem < 0) {
      throw new Error("Failed to send response downstream");
    }
    */
  }

  private _headers: HeadersImpl = new HeadersImpl();
  private _body: string;
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

  body(): string {
    /*
    if (this._body) {
      return this._body;
    }

    const resp_json: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };
    const resp_str = JSON.stringify(resp_json);
    const resp_mem = Memory.fromString(resp_str);
    const ret_mem = _apoxy_resp_body(resp_mem.offset);
    if (ret_mem <= 0) {
      throw new Error("Failed to get response body");
    }
    this._body = Memory.find(ret_mem).readString();
    this.content_len = this._body.length;
    */
    return this._body;
  }

  send(body: string): void {
    this.content_len = body.length;
    this._body = body;
  }

  sendDownstream(): void {
    /*
    console.log("Sending backend response downstream");
    const resp_json: ResponseABI = {
      status_code: this.code,
      content_len: this.content_len,
      header: this._headers.toObject(),
    };
    const resp_str = JSON.stringify(resp_json);
    // For the upstream case, we're modifying the response object in place
    // so we don't need to send the body to the VM.
    const ret_mem = _apoxy_resp_send(
      Memory.fromString(resp_str),
      Memory.fromString(this._body).offset,
    );
    if (ret_mem <= 0) {
      throw new Error("Failed to send response downstream");
    }
    */
  }

  private _headers: Headers = new HeadersImpl();
  private _body: string = "";
}

export {};
