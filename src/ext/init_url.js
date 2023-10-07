import * as webidl from 'ext:deno_webidl/00_webidl.js';
import * as url from 'ext:deno_url/00_url.js';
import * as urlPattern from 'ext:deno_url/01_urlpattern.js';

import { applyToGlobal, nonEnumerable } from 'ext:js_playground/js_playground.js';
applyToGlobal({
    URL: nonEnumerable(url.URL),
    URLPattern: nonEnumerable(urlPattern.URLPattern),
    URLSearchParams: nonEnumerable(url.URLSearchParams),
    // Branding as a WebIDL object
    [webidl.brand]: nonEnumerable(webidl.brand),
});