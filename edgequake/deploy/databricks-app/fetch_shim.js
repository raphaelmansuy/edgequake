(function () {
  // The browser gateway closed a 100KB JSON document after 16375 of 103316
  // bytes, and a notebook client closed a 128KiB body at 81920. Markdown is
  // posted with fetch, not FormData, so both paths stay under an 8KiB slice.
  var SLICE = 8 * 1024;
  var nativeFetch = window.fetch;
  var encoder = typeof TextEncoder !== "undefined" ? new TextEncoder() : null;

  function uploadId() {
    var id = Math.random().toString(36).slice(2) + Date.now().toString(36);
    id = id.replace(/[^A-Za-z0-9]/g, "");
    while (id.length < 16) id += "0";
    return id.slice(0, 64);
  }

  function asRawSource(body) {
    if (typeof body === "string") {
      if (!encoder) return null;
      var encoded = encoder.encode(body);
      return {
        size: encoded.byteLength,
        typeHint: "application/json",
        slice: function (start, end) {
          return encoded.slice(start, end);
        },
      };
    }
    if (typeof Blob !== "undefined" && body instanceof Blob) {
      return {
        size: body.size,
        typeHint: body.type || "application/octet-stream",
        slice: function (start, end) {
          return body.slice(start, end);
        },
      };
    }
    if (body instanceof ArrayBuffer) {
      var view = new Uint8Array(body);
      return {
        size: view.byteLength,
        typeHint: "application/octet-stream",
        slice: function (start, end) {
          return view.slice(start, end);
        },
      };
    }
    if (ArrayBuffer.isView(body)) {
      var typed = new Uint8Array(body.buffer, body.byteOffset, body.byteLength);
      return {
        size: typed.byteLength,
        typeHint: "application/octet-stream",
        slice: function (start, end) {
          return typed.slice(start, end);
        },
      };
    }
    return null;
  }

  function oneFormFile(body) {
    if (typeof FormData === "undefined" || !(body instanceof FormData)) return null;
    var file = null;
    var field = "file";
    var fields = {};
    var files = 0;
    body.forEach(function (value, key) {
      var isFile =
        value &&
        typeof value.size === "number" &&
        typeof value.slice === "function" &&
        typeof value !== "string";
      if (isFile) {
        files += 1;
        file = value;
        field = key;
      } else {
        fields[key] = String(value == null ? "" : value);
      }
    });
    if (files !== 1 || !file || file.size <= SLICE) return null;
    return { file: file, field: field, fields: fields };
  }

  function headerBag(input, init) {
    try {
      return new Headers((init && init.headers) || (input && input.headers) || undefined);
    } catch (error) {
      return new Headers();
    }
  }

  function requestUrl(input) {
    if (typeof input === "string") return input;
    if (input && typeof input.url === "string") return input.url;
    return String(input);
  }

  function requestMethod(input, init) {
    var method = (init && init.method) || (input && input.method) || "GET";
    return String(method).toUpperCase();
  }

  function baseHeadersFrom(input, init) {
    var source = headerBag(input, init);
    var headers = new Headers();
    source.forEach(function (value, key) {
      var lower = String(key).toLowerCase();
      if (lower === "content-length" || lower === "content-type") return;
      if (lower.indexOf("x-edgequake-upload-") === 0) return;
      headers.set(key, value);
    });
    return headers;
  }

  function contentTypeOf(input, init, fallback) {
    try {
      var declared = headerBag(input, init).get("content-type");
      if (declared) return declared;
    } catch (error) {}
    return fallback || "application/octet-stream";
  }

  function runFetchSlices(url, method, baseHeaders, credentials, signal, total, description, readPiece, onAdvance) {
    var id = uploadId();
    var offset = 0;
    function headers(extra) {
      var copied = new Headers(baseHeaders);
      Object.keys(extra).forEach(function (name) {
        copied.set(name, extra[name]);
      });
      return copied;
    }
    function post(body, extra) {
      var req = {
        method: method,
        headers: headers(extra),
        body: body,
        credentials: credentials || "same-origin",
      };
      if (signal) req.signal = signal;
      return nativeFetch(url, req);
    }
    function sendSlice() {
      var end = Math.min(offset + SLICE, total);
      var at = offset;
      return post(readPiece(at, end), {
        "Content-Type": "application/octet-stream",
        "X-EdgeQuake-Upload-Id": id,
        "X-EdgeQuake-Upload-Role": "bytes",
        "X-EdgeQuake-Upload-Offset": String(at),
        "X-EdgeQuake-Upload-Total": String(total),
      }).then(function (response) {
        offset = end;
        if (onAdvance) onAdvance(offset, total);
        if (offset < total) {
          if (response.status !== 202) return response;
          return response.text().then(sendSlice);
        }
        return response;
      });
    }
    return post(JSON.stringify(description), {
      "Content-Type": "application/json",
      "X-EdgeQuake-Upload-Id": id,
      "X-EdgeQuake-Upload-Role": "fields",
    }).then(function (response) {
      if (response.status !== 202) return response;
      return response.text().then(sendSlice);
    });
  }

  if (nativeFetch && !nativeFetch.__eq) {
    function wrapped(input, init) {
      init = Object.assign({}, init || {});
      var headers = headerBag(input, init);
      try {
        var authorization = headers.get("Authorization");
        if (authorization && !headers.get("X-EdgeQuake-Authorization")) {
          headers.set("X-EdgeQuake-Authorization", authorization);
          init.headers = headers;
        }
      } catch (error) {}
      if (headers.get("X-EdgeQuake-Upload-Id")) {
        return nativeFetch.call(this, input, init);
      }
      var method = requestMethod(input, init);
      if (method === "GET" || method === "HEAD") {
        return nativeFetch.call(this, input, init);
      }
      var form = oneFormFile(init.body);
      if (form) {
        return runFetchSlices(
          requestUrl(input),
          method,
          baseHeadersFrom(input, init),
          init.credentials || "same-origin",
          init.signal,
          form.file.size,
          {
            filename: form.file.name || "upload",
            field: form.field,
            content_type: form.file.type || "application/octet-stream",
            total: form.file.size,
            fields: form.fields,
          },
          function (start, end) {
            return form.file.slice(start, end);
          }
        );
      }
      var source = asRawSource(init.body);
      if (!source || source.size <= SLICE) {
        return nativeFetch.call(this, input, init);
      }
      return runFetchSlices(
        requestUrl(input),
        method,
        baseHeadersFrom(input, init),
        init.credentials || "same-origin",
        init.signal,
        source.size,
        {
          mode: "raw",
          total: source.size,
          content_type: contentTypeOf(input, init, source.typeHint),
        },
        source.slice
      );
    }
    wrapped.__eq = 1;
    window.fetch = wrapped;
  }

  var proto = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  if (!proto || proto.__eqAuth) return;

  var nativeOpen = proto.open;
  var nativeSet = proto.setRequestHeader;
  var nativeSend = proto.send;
  var canShadowStatus = false;
  try {
    var probeXhr = new XMLHttpRequest();
    Object.defineProperty(probeXhr, "status", {
      configurable: true,
      get: function () {
        return 201;
      },
    });
    canShadowStatus = probeXhr.status === 201;
  } catch (error) {
    canShadowStatus = false;
  }

  proto.open = function (method, url) {
    this.__eqMethod = method;
    this.__eqUrl = url;
    this.__eqHeaders = [];
    this.__eqSet = 0;
    return nativeOpen.apply(this, arguments);
  };

  proto.setRequestHeader = function (name, value) {
    var lower = String(name).toLowerCase();
    if (lower === "x-edgequake-authorization") this.__eqSet = 1;
    try {
      nativeSet.call(this, name, value);
    } catch (error) {
      if (lower !== "x-edgequake-authorization") throw error;
    }
    if (!this.__eqHeaders) this.__eqHeaders = [];
    this.__eqHeaders.push([name, value]);
    if (lower === "authorization" && value && !this.__eqSet) {
      this.__eqSet = 1;
      try {
        nativeSet.call(this, "X-EdgeQuake-Authorization", value);
      } catch (error) {}
      this.__eqHeaders.push(["X-EdgeQuake-Authorization", value]);
    }
  };

  function report(xhr, loaded, total) {
    try {
      if (xhr.upload && typeof xhr.upload.onprogress === "function") {
        xhr.upload.onprogress({
          lengthComputable: total > 0,
          loaded: loaded,
          total: total,
        });
      }
    } catch (error) {}
  }

  function finish(xhr, status, statusText, text, headers) {
    Object.defineProperty(xhr, "status", {
      configurable: true,
      get: function () {
        return status;
      },
    });
    Object.defineProperty(xhr, "statusText", {
      configurable: true,
      get: function () {
        return statusText || "";
      },
    });
    Object.defineProperty(xhr, "responseText", {
      configurable: true,
      get: function () {
        return text;
      },
    });
    Object.defineProperty(xhr, "readyState", {
      configurable: true,
      get: function () {
        return 4;
      },
    });
    xhr.getResponseHeader = function (name) {
      try {
        return headers.get(name);
      } catch (error) {
        return null;
      }
    };
    if (typeof xhr.onload === "function") xhr.onload({ target: xhr, type: "load" });
  }

  function storedHeaders(xhr) {
    var headers = new Headers();
    var stored = xhr.__eqHeaders || [];
    for (var i = 0; i < stored.length; i++) {
      var name = stored[i][0];
      var lower = String(name).toLowerCase();
      if (lower === "content-type" || lower === "content-length") continue;
      headers.set(name, stored[i][1]);
    }
    return headers;
  }

  function xhrContentType(xhr, fallback) {
    var stored = xhr.__eqHeaders || [];
    for (var i = 0; i < stored.length; i++) {
      if (String(stored[i][0]).toLowerCase() === "content-type") return stored[i][1];
    }
    return fallback || "application/json";
  }

  function bindXhr(xhr, total, description, readPiece) {
    var timedOut = false;
    var timer = 0;
    var controller = new AbortController();
    if (xhr.timeout && xhr.timeout > 0) {
      timer = setTimeout(function () {
        timedOut = true;
        controller.abort();
        if (typeof xhr.ontimeout === "function") xhr.ontimeout();
      }, xhr.timeout);
    }
    runFetchSlices(
      xhr.__eqUrl,
      xhr.__eqMethod || "POST",
      storedHeaders(xhr),
      "same-origin",
      controller.signal,
      total,
      description,
      readPiece,
      function (loaded) {
        report(xhr, loaded, total);
      }
    )
      .then(function (response) {
        return response.text().then(function (text) {
          if (timer) clearTimeout(timer);
          if (timedOut) return;
          finish(xhr, response.status, response.statusText, text, response.headers);
        });
      })
      .catch(function (error) {
        if (timer) clearTimeout(timer);
        if (timedOut) return;
        try {
          xhr.__eqErr = error && error.message ? String(error.message) : String(error);
        } catch (ignored) {}
        if (typeof xhr.onerror === "function") xhr.onerror();
      });
  }

  proto.send = function (body) {
    if (!canShadowStatus) return nativeSend.call(this, body);
    var form = oneFormFile(body);
    if (form) {
      bindXhr(
        this,
        form.file.size,
        {
          filename: form.file.name || "upload",
          field: form.field,
          content_type: form.file.type || "application/octet-stream",
          total: form.file.size,
          fields: form.fields,
        },
        function (start, end) {
          return form.file.slice(start, end);
        }
      );
      return;
    }
    var source = asRawSource(body);
    if (source && source.size > SLICE) {
      bindXhr(
        this,
        source.size,
        {
          mode: "raw",
          total: source.size,
          content_type: xhrContentType(this, source.typeHint),
        },
        source.slice
      );
      return;
    }
    return nativeSend.call(this, body);
  };
  proto.__eqAuth = 1;
})();
