(function () {
  var nativeFetch = window.fetch;
  if (nativeFetch && !nativeFetch.__eq) {
    function wrapped(input, init) {
      init = init || {};
      try {
        var headers = new Headers(init.headers || (input && input.headers));
        var authorization = headers.get("Authorization");
        if (authorization && !headers.get("X-EdgeQuake-Authorization")) {
          headers.set("X-EdgeQuake-Authorization", authorization);
          init.headers = headers;
        }
      } catch (error) {}
      return nativeFetch.call(this, input, init);
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

  // Stay under the Apps gateway Content-Length cutoff. 1.5MB bodies are closed
  // early; slices this size are forwarded whole.
  var SLICE = 128 * 1024;

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

  proto.send = function (body) {
    var form = typeof FormData !== "undefined" && body instanceof FormData;
    if (!form || !canShadowStatus) return nativeSend.call(this, body);
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
    if (files !== 1 || !file || file.size <= SLICE) return nativeSend.call(this, body);
    sliceUpload(this, file, field, fields);
  };
  proto.__eqAuth = 1;

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

  function uploadId() {
    var id = Math.random().toString(36).slice(2) + Date.now().toString(36);
    id = id.replace(/[^A-Za-z0-9]/g, "");
    while (id.length < 16) id += "0";
    return id.slice(0, 64);
  }

  function storedHeaders(xhr, extra) {
    var headers = new Headers();
    var stored = xhr.__eqHeaders || [];
    for (var i = 0; i < stored.length; i++) {
      var name = stored[i][0];
      var lower = String(name).toLowerCase();
      if (lower === "content-type" || lower === "content-length") continue;
      headers.set(name, stored[i][1]);
    }
    Object.keys(extra).forEach(function (name) {
      headers.set(name, extra[name]);
    });
    return headers;
  }

  function sliceUpload(xhr, file, field, fields) {
    var id = uploadId();
    var total = file.size;
    var offset = 0;
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
    function fail(error) {
      if (timer) clearTimeout(timer);
      if (timedOut) return;
      try {
        xhr.__eqErr = error && error.message ? String(error.message) : String(error);
      } catch (ignored) {}
      if (typeof xhr.onerror === "function") xhr.onerror();
    }
    function post(body, extra) {
      return window.fetch(xhr.__eqUrl, {
        method: xhr.__eqMethod || "POST",
        headers: storedHeaders(xhr, extra),
        body: body,
        credentials: "same-origin",
        signal: controller.signal,
      });
    }
    post(
      JSON.stringify({
        filename: file.name || "upload",
        field: field,
        content_type: file.type || "application/octet-stream",
        total: total,
        fields: fields,
      }),
      {
        "Content-Type": "application/json",
        "X-EdgeQuake-Upload-Id": id,
        "X-EdgeQuake-Upload-Role": "fields",
      }
    )
      .then(function (response) {
        return response.text().then(function (text) {
          if (response.status !== 202) {
            if (timer) clearTimeout(timer);
            finish(xhr, response.status, response.statusText, text, response.headers);
            return;
          }
          return sendSlice();
        });
      })
      .catch(fail);

    function sendSlice() {
      if (timedOut) return;
      var end = Math.min(offset + SLICE, total);
      var slice = file.slice(offset, end);
      var at = offset;
      return post(slice, {
        "Content-Type": "application/octet-stream",
        "X-EdgeQuake-Upload-Id": id,
        "X-EdgeQuake-Upload-Role": "bytes",
        "X-EdgeQuake-Upload-Offset": String(at),
        "X-EdgeQuake-Upload-Total": String(total),
      })
        .then(function (response) {
          return response.text().then(function (text) {
            offset = end;
            report(xhr, offset, total);
            var last = offset >= total;
            if (!last && response.status === 202) return sendSlice();
            if (timer) clearTimeout(timer);
            if (timedOut) return;
            finish(xhr, response.status, response.statusText, text, response.headers);
          });
        })
        .catch(fail);
    }
  }
})();
