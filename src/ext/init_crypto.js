import * as crypto from "ext:deno_crypto/00_crypto.js";

import { applyToGlobal, nonEnumerable, readOnly } from 'ext:js_playground/js_playground.js';
applyToGlobal({
    CryptoKey: nonEnumerable(crypto.CryptoKey),
    crypto: readOnly(crypto.crypto),
    Crypto: nonEnumerable(crypto.Crypto),
    SubtleCrypto: nonEnumerable(crypto.SubtleCrypto),
});