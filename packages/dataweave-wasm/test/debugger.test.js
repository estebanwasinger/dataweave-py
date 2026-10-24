const test = require("node:test");
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { DebugAdapter } = require("..");
const transcript = JSON.parse(readFileSync(require("node:path").join(__dirname, "../../../tests/fixtures/debugger/session.json"), "utf8"));
function until(adapter, event) { const events = []; for (let i = 0; i < 10000; i++) { events.push(...adapter.poll(7)); if (events.some(m => m.event === event)) return events; } assert.fail(`No ${event} event`); }
test("shared DAP transcript executes inside WASM without replay", () => {
  const adapter = new DebugAdapter();
  try {
    for (const request of transcript) {
      const messages = adapter.dispatch(request);
      assert.equal(messages[0].success, true, JSON.stringify(messages));
      assert.equal(messages[0].request_seq, request.seq);
      if (request.command === "evaluate") assert.equal(messages[0].body.result, "2");
      if (["configurationDone", "stepIn"].includes(request.command)) until(adapter, "stopped");
      if (request.command === "continue") {
        const messages = until(adapter, "terminated");
        const result = messages.find(m => m.body?.data?.kind === "result");
        assert.deepEqual(JSON.parse(result.body.data.result), [2,4,6]);
        assert.equal(messages.filter(m => m.event === "terminated").length, 1);
      }
    }
  } finally { adapter.dispose(); }
});
test("WASM debugger rejects unsupported code before executing logs", () => {
  const adapter = new DebugAdapter();
  try {
    adapter.dispatch({ seq: 1, type: "request", command: "initialize" });
    const messages = adapter.dispatch({ seq: 2, type: "request", command: "launch", arguments: { source: "%dw 2.0\nvar x=log(1)\n---\ntry(() -> payload)" } });
    assert.equal(messages[0].success, false);
    assert.match(messages[0].message, /not supported by the debugger/);
    assert.deepEqual(adapter.poll(), []);
  } finally { adapter.dispose(); }
});
test("WASM update cases pause at replacement expressions with scoped bindings", () => {
  const adapter = new DebugAdapter();
  let seq = 0;
  const request = (command, args = {}) => {
    const messages = adapter.dispatch({ seq: ++seq, type: "request", command, arguments: args });
    assert.equal(messages[0].success, true, JSON.stringify(messages));
    return messages[0].body;
  };
  try {
    request("initialize");
    request("launch", { source: "%dw 2.0\noutput application/json\n---\n{numA:2} update {\ncase .numA -> $ * 2\n}", stopOnEntry: false });
    const bps = request("setBreakpoints", { source: { sourceReference: 1 }, breakpoints: [{line:5}] });
    assert.equal(bps.breakpoints[0].verified, true);
    request("configurationDone");
    const stopped = until(adapter, "stopped");
    assert.equal(stopped.find(m => m.event === "stopped").body.reason, "breakpoint");
    const frame = request("stackTrace", {threadId:1}).stackFrames[0];
    assert.equal(frame.line, 5);
    assert.equal(request("evaluate", {frameId:frame.id,expression:"$",context:"watch"}).result, "2");
    request("continue", {threadId:1});
    const result = until(adapter, "terminated").find(m => m.body?.data?.kind === "result");
    assert.deepEqual(JSON.parse(result.body.data.result), {numA:4});
  } finally { adapter.dispose(); }
});
test("WASM resolves typed and arity overloads including recursive dispatch", () => {
  const adapter = new DebugAdapter();
  let seq = 0;
  const request = (command, args = {}) => {
    const messages = adapter.dispatch({ seq: ++seq, type: "request", command, arguments: args });
    assert.equal(messages[0].success, true, JSON.stringify(messages));
    return messages[0].body;
  };
  try {
    request("initialize");
    request("launch", { source: "%dw 2.0\noutput application/json\nfun multiply(n,m:Number)=n*m*2\nfun multiply(n,m:String)=n*m as Number\nfun multiply(n)=multiply(n,n)\n---\n[multiply(2,4),multiply(2,\"4\"),multiply(2)]", stopOnEntry: false });
    request("configurationDone");
    const result = until(adapter, "terminated").find(m => m.body?.data?.kind === "result");
    assert.deepEqual(JSON.parse(result.body.data.result), [16,8,8]);
  } finally { adapter.dispose(); }
});
test("M1 grammar shared DAP transcript executes through WASM", () => {
  const requests=JSON.parse(readFileSync(require("node:path").join(__dirname,"../../../tests/fixtures/debugger/m1.json"),"utf8"));
  const adapter=new DebugAdapter();
  try {
    for(const request of requests){
      const messages=adapter.dispatch(request);
      assert.equal(messages[0].success,true,JSON.stringify(messages));
      if(request.command==="evaluate")assert.equal(messages[0].body.result,"2");
      if(["configurationDone","stepIn"].includes(request.command))until(adapter,"stopped");
      if(request.command==="continue"){
        const result=until(adapter,"terminated").find(m=>m.body?.data?.kind==="result");
        assert.deepEqual(JSON.parse(result.body.data.result),{result:{k2:"3"},filtered:[2,3],replaced:"a10b221",closure:5});
      }
    }
  }finally{adapter.dispose();}
});

test("M2 callback families share one live DAP conversation", () => {
  const fixture = name => JSON.parse(readFileSync(require("node:path").join(__dirname,"../../../tests/fixtures/debugger",name),"utf8"));
  const adapter = new DebugAdapter();
  try {
    for (const request of fixture("m2.json")) {
      const response=adapter.dispatch(request)[0];
      assert.equal(response.success,true,JSON.stringify(response));
      if(request.command==="evaluate") assert.equal(response.body.result,"2");
      if(["configurationDone","stepIn"].includes(request.command)) until(adapter,"stopped");
      if(request.command==="continue") {
        const messages=until(adapter,"terminated");
        const result=messages.find(m=>m.body?.data?.kind==="result");
        assert.deepEqual(JSON.parse(result.body.data.result),fixture("m2.expected.json"));
        assert.equal(messages.filter(m=>m.body?.data?.kind==="log").length,1);
      }
    }
  } finally {adapter.dispose();}
});
