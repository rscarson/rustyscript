import * as get_value from '../get_value.ts';

// We will get the value set up for us by the runtime, and transform it
// into a string!
let value = get_value.getValue();
export const final_value = `${value.toFixed(2)}`;