import "ext:deno_web/01_dom_exception.js";
import * as timers from "ext:deno_web/02_timers.js";

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({    
  clearInterval: nonEnumerable(timers.clearInterval),
  clearTimeout: nonEnumerable(timers.clearTimeout),
  
  setImmediate: nonEnumerable(timers.setImmediate),
  setInterval: nonEnumerable(timers.setInterval),
  setTimeout: nonEnumerable(timers.setTimeout),
});