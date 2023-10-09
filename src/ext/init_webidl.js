import * as webidl from 'ext:deno_webidl/00_webidl.js';

import { applyToGlobal, nonEnumerable } from 'ext:js_playground/js_playground.js';
applyToGlobal({
    // Branding as a WebIDL object
    [webidl.brand]: nonEnumerable(webidl.brand),
});