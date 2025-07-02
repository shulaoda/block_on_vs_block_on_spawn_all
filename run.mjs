import { Bench } from 'tinybench';
import { runWithJoin, runWithBlock } from "./index.js";

async function runner(v, k) {
    return Promise.resolve(v + k)
}

const bench = new Bench({ time: 100 });

bench.add('runWithJoin', async () => {
    await runWithJoin(runner);
});

bench.add('runWithBlock', async () => {
    await runWithBlock(runner);
});

await bench.run();
console.table(bench.table());