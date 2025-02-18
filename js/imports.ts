
import { join } from './operator/join.ts';
import { string2float } from './cast/string2float.ts';
import { int2string } from './cast/int2string.ts';
import { float2string } from './cast/float2string.ts';
import { say_int } from './looks/say_int.ts';
import { say_string } from './looks/say_string.ts';
import { say_float } from './looks/say_float.ts';
export const imports = {
    cast: { string2float, int2string, float2string },
	looks: { say_int, say_string, say_float },
	operator: { join }
};
            