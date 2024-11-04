// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

import * as event from 'ext:deno_web/02_event.js';
import { DOMException } from "ext:deno_web/01_dom_exception.js";
import { core, primordials, internals } from "ext:core/mod.js";
const { BadResource, Interrupted, NotCapable } = core;
const {
    Error,
    ErrorPrototype,
    ObjectPrototypeIsPrototypeOf,
  } = primordials;

  import {
    getDefaultInspectOptions,
    getStderrNoColor,
    inspectArgs,
    quoteString,
  } from "ext:deno_console/01_console.js";

import { op_set_format_exception_callback } from "ext:core/ops";

class NotFound extends Error {
  constructor(msg) {
    super(msg);
    this.name = "NotFound";
  }
}

class ConnectionRefused extends Error {
  constructor(msg) {
    super(msg);
    this.name = "ConnectionRefused";
  }
}

class ConnectionReset extends Error {
  constructor(msg) {
    super(msg);
    this.name = "ConnectionReset";
  }
}

class ConnectionAborted extends Error {
  constructor(msg) {
    super(msg);
    this.name = "ConnectionAborted";
  }
}

class NotConnected extends Error {
  constructor(msg) {
    super(msg);
    this.name = "NotConnected";
  }
}

class AddrInUse extends Error {
  constructor(msg) {
    super(msg);
    this.name = "AddrInUse";
  }
}

class AddrNotAvailable extends Error {
  constructor(msg) {
    super(msg);
    this.name = "AddrNotAvailable";
  }
}

class BrokenPipe extends Error {
  constructor(msg) {
    super(msg);
    this.name = "BrokenPipe";
  }
}

class AlreadyExists extends Error {
  constructor(msg) {
    super(msg);
    this.name = "AlreadyExists";
  }
}

class InvalidData extends Error {
  constructor(msg) {
    super(msg);
    this.name = "InvalidData";
  }
}

class TimedOut extends Error {
  constructor(msg) {
    super(msg);
    this.name = "TimedOut";
  }
}

class WriteZero extends Error {
  constructor(msg) {
    super(msg);
    this.name = "WriteZero";
  }
}

class WouldBlock extends Error {
  constructor(msg) {
    super(msg);
    this.name = "WouldBlock";
  }
}

class UnexpectedEof extends Error {
  constructor(msg) {
    super(msg);
    this.name = "UnexpectedEof";
  }
}

class Http extends Error {
  constructor(msg) {
    super(msg);
    this.name = "Http";
  }
}

class Busy extends Error {
  constructor(msg) {
    super(msg);
    this.name = "Busy";
  }
}

class PermissionDenied extends Error {
  constructor(msg) {
    super(msg);
    this.name = "PermissionDenied";
  }
}

class NotSupported extends Error {
  constructor(msg) {
    super(msg);
    this.name = "NotSupported";
  }
}

class FilesystemLoop extends Error {
  constructor(msg) {
    super(msg);
    this.name = "FilesystemLoop";
  }
}

class IsADirectory extends Error {
  constructor(msg) {
    super(msg);
    this.name = "IsADirectory";
  }
}

class NetworkUnreachable extends Error {
  constructor(msg) {
    super(msg);
    this.name = "NetworkUnreachable";
  }
}

class NotADirectory extends Error {
  constructor(msg) {
    super(msg);
    this.name = "NotADirectory";
  }
}

const errors = {
  NotFound,
  PermissionDenied,
  ConnectionRefused,
  ConnectionReset,
  ConnectionAborted,
  NotConnected,
  AddrInUse,
  AddrNotAvailable,
  BrokenPipe,
  AlreadyExists,
  InvalidData,
  TimedOut,
  Interrupted,
  WriteZero,
  WouldBlock,
  UnexpectedEof,
  BadResource,
  Http,
  Busy,
  NotSupported,
  FilesystemLoop,
  IsADirectory,
  NetworkUnreachable,
  NotADirectory,
  NotCapable,
};

import { DedicatedWorkerGlobalScope } from 'ext:deno_web/04_global_interfaces.js';
primordials.ObjectSetPrototypeOf(globalThis, DedicatedWorkerGlobalScope.prototype);
event.saveGlobalThisReference(globalThis);
event.setEventTargetData(globalThis);

let globalThis_;
globalThis_ = globalThis;

core.registerErrorClass("NotFound", errors.NotFound);
core.registerErrorClass("ConnectionRefused", errors.ConnectionRefused);
core.registerErrorClass("ConnectionReset", errors.ConnectionReset);
core.registerErrorClass("ConnectionAborted", errors.ConnectionAborted);
core.registerErrorClass("NotConnected", errors.NotConnected);
core.registerErrorClass("AddrInUse", errors.AddrInUse);
core.registerErrorClass("AddrNotAvailable", errors.AddrNotAvailable);
core.registerErrorClass("BrokenPipe", errors.BrokenPipe);
core.registerErrorClass("PermissionDenied", errors.PermissionDenied);
core.registerErrorClass("AlreadyExists", errors.AlreadyExists);
core.registerErrorClass("InvalidData", errors.InvalidData);
core.registerErrorClass("TimedOut", errors.TimedOut);
core.registerErrorClass("WouldBlock", errors.WouldBlock);
core.registerErrorClass("WriteZero", errors.WriteZero);
core.registerErrorClass("UnexpectedEof", errors.UnexpectedEof);
core.registerErrorClass("Http", errors.Http);
core.registerErrorClass("Busy", errors.Busy);
core.registerErrorClass("NotSupported", errors.NotSupported);
core.registerErrorClass("FilesystemLoop", errors.FilesystemLoop);
core.registerErrorClass("IsADirectory", errors.IsADirectory);
core.registerErrorClass("NetworkUnreachable", errors.NetworkUnreachable);
core.registerErrorClass("NotADirectory", errors.NotADirectory);
core.registerErrorBuilder(
  "DOMExceptionOperationError",
  function DOMExceptionOperationError(msg) {
    return new DOMException(msg, "OperationError");
  },
);
core.registerErrorBuilder(
  "DOMExceptionQuotaExceededError",
  function DOMExceptionQuotaExceededError(msg) {
    return new DOMException(msg, "QuotaExceededError");
  },
);
core.registerErrorBuilder(
  "DOMExceptionNotSupportedError",
  function DOMExceptionNotSupportedError(msg) {
    return new DOMException(msg, "NotSupported");
  },
);
core.registerErrorBuilder(
  "DOMExceptionNetworkError",
  function DOMExceptionNetworkError(msg) {
    return new DOMException(msg, "NetworkError");
  },
);
core.registerErrorBuilder(
  "DOMExceptionAbortError",
  function DOMExceptionAbortError(msg) {
    return new DOMException(msg, "AbortError");
  },
);
core.registerErrorBuilder(
  "DOMExceptionInvalidCharacterError",
  function DOMExceptionInvalidCharacterError(msg) {
    return new DOMException(msg, "InvalidCharacterError");
  },
);
core.registerErrorBuilder(
  "DOMExceptionDataError",
  function DOMExceptionDataError(msg) {
    return new DOMException(msg, "DataError");
  },
);

core.setUnhandledPromiseRejectionHandler(processUnhandledPromiseRejection);
core.setHandledPromiseRejectionHandler(processRejectionHandled);

// Notification that the core received an unhandled promise rejection that is about to
// terminate the runtime. If we can handle it, attempt to do so.
function processUnhandledPromiseRejection(promise, reason) {
  const rejectionEvent = new event.PromiseRejectionEvent(
    "unhandledrejection",
    {
      cancelable: true,
      promise,
      reason,
    },
  );

  // Note that the handler may throw, causing a recursive "error" event
  globalThis_.dispatchEvent(rejectionEvent);

  // If event was not yet prevented, try handing it off to Node compat layer
  // (if it was initialized)
  if (
    !rejectionEvent.defaultPrevented &&
    typeof internals.nodeProcessUnhandledRejectionCallback !== "undefined"
  ) {
    internals.nodeProcessUnhandledRejectionCallback(rejectionEvent);
  }

  // If event was not prevented (or "unhandledrejection" listeners didn't
  // throw) we will let Rust side handle it.
  if (rejectionEvent.defaultPrevented) {
    return true;
  }

  return false;
}

function processRejectionHandled(promise, reason) {
  const rejectionHandledEvent = new event.PromiseRejectionEvent(
    "rejectionhandled",
    { promise, reason },
  );

  // Note that the handler may throw, causing a recursive "error" event
  globalThis_.dispatchEvent(rejectionHandledEvent);

  if (typeof internals.nodeProcessRejectionHandledCallback !== "undefined") {
    internals.nodeProcessRejectionHandledCallback(rejectionHandledEvent);
  }
}

function formatException(error) {
    if (
      core.isNativeError(error) ||
      ObjectPrototypeIsPrototypeOf(ErrorPrototype, error)
    ) {
      return null;
    } else if (typeof error == "string") {
      return `Uncaught ${
        inspectArgs([quoteString(error, getDefaultInspectOptions())], {
          colors: !getStderrNoColor(),
        })
      }`;
    } else if (ObjectPrototypeIsPrototypeOf(ErrorEvent.prototype, error)) {
        /*
        Need to process ErrorEvent here into an exception string
        */
       console.log(JSON.stringify(error.error));
       return formatException(error.error);

    } else {
      return `Uncaught ${inspectArgs([error], { colors: !getStderrNoColor() })}`;
    }
  }

  
  core.setReportExceptionCallback(event.reportException);
  op_set_format_exception_callback(formatException);