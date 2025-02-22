
import { lt_string } from './operator/lt_string.ts';
import { eq_string } from './operator/eq_string.ts';
import { join } from './operator/join.ts';
import { gt_string } from './operator/gt_string.ts';
import { dayssince2000 } from './sensing/dayssince2000.ts';
import { string2float } from './cast/string2float.ts';
import { int2string } from './cast/int2string.ts';
import { float2string } from './cast/float2string.ts';
import { say_int } from './looks/say_int.ts';
import { say_string } from './looks/say_string.ts';
import { say_float } from './looks/say_float.ts';
export const imports = {
    looks: { say_int, say_string, say_float },
	operator: { lt_string, eq_string, join, gt_string },
	sensing: { dayssince2000 },
	cast: { string2float, int2string, float2string }
};
            