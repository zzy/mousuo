// mousuo Q1/Q2/Q3/Q6 冒烟验收（注册表单无 topics 字段）
import { Buffer } from "node:buffer";
import { smokeConfig } from "./smoke_config.mjs";

const CFG = smokeConfig("./.env", "mousuo", "mousuo");

let failures = 0;
function check(name, cond, extra = "") {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}${cond ? "" : "  << " + extra}`);
  if (!cond) failures++;
}

class Jar {
  constructor() { this.map = new Map(); }
  ingest(res) {
    for (const c of res.headers.getSetCookie()) {
      const kv = c.split(";")[0];
      const i = kv.indexOf("=");
      if (i > 0) this.map.set(kv.slice(0, i), kv.slice(i + 1));
    }
  }
  header() { return [...this.map].map(([k, v]) => `${k}=${v}`).join("; "); }
}

async function get(path, jar = new Jar(), { expect = 200 } = {}) {
  const res = await fetch(CFG.base + path, { redirect: "manual", headers: { cookie: jar.header() } });
  jar.ingest(res);
  const body = await res.text();
  if (res.status !== expect && expect !== null) {
    console.log(`  [warn] GET ${path} -> ${res.status}, want ${expect}`);
  }
  return { res, body, jar };
}

async function post(path, fields, jar) {
  const res = await fetch(CFG.base + path, {
    method: "POST",
    redirect: "manual",
    headers: { cookie: jar.header(), "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams(fields),
  });
  jar.ingest(res);
  const body = await res.text();
  return { res, body, jar };
}

const csrf = (html) => html.match(/name="csrf_token" value="([^"]*)"/)?.[1] ?? "";
const jsCap = (html) => html.match(/ok=v==='([^']*)'/)?.[1] ?? "";
const svgCap = (html) => {
  const m = html.match(/data:image\/svg\+xml;base64,([A-Za-z0-9+/=]+)/);
  if (!m) return "";
  const svg = Buffer.from(m[1], "base64").toString("utf8");
  const texts = [...svg.matchAll(/<text[^>]*font-weight="bold"[^>]*>([^<]*)<\/text>/g)].map((x) => x[1]);
  if (texts.length < 3) return "";
  const [l, op, r] = texts;
  return String(op === "+" ? Number(l) + Number(r) : Number(l) - Number(r));
};
const loc = (r) => r.res.headers.get("location") || "";

async function dbq(sql) {
  const res = await fetch(CFG.db, {
    method: "POST",
    headers: {
      authorization: CFG.dbAuth,
      "surreal-ns": CFG.ns,
      "surreal-db": CFG.dbName,
      accept: "application/json",
      "content-type": "text/plain",
    },
    body: sql,
  });
  const j = await res.json();
  return j?.[0]?.result?.[0] ?? null;
}

const U = "smoke" + Date.now().toString(36).slice(-8);
const P1 = "Smoke123a";
const P2 = "Smoke999a";
const P3 = "Smoke888a";
console.log("user:", U);

// ── 1. 注册（待激活） ──────────────────────────────────────────────
let jar = new Jar();
let r = await get("/zh/register", jar);
let f = { username: U, email: `${U}@example.com`, password: P1, confirm_password: P1,
  introduction: "## About", captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/register", f, jar);
check("注册 303 → success=1", r.res.status === 303 && loc(r).includes("success=1"), `${r.res.status} ${loc(r)}`);
check("注册成功页带 mail=1（SMTP 凭据仍失败，属预期）", loc(r).includes("mail=1"), loc(r));

// ── 2. 未激活签入 → not_activation + 重发链接 ──────────────────────
let j2 = new Jar();
r = await get("/zh/sign-in", j2);
f = { account: U, password: P1, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, j2);
check("未激活签入 303 → error=not_activation&account", r.res.status === 303 && loc(r).includes("error=not_activation") && loc(r).includes(`account=${U}`), loc(r));
r = await get("/zh/sign-in?error=not_activation&account=" + U, j2);
check("签入页展示重发链接", r.body.includes(`/zh/users/resend?account=${U}`), "resend link missing");

// ── 3. Q1 重发：token 轮换 ────────────────────────────────────────
let j3 = new Jar();
r = await get(`/zh/users/resend?account=${U}`, j3);
const tokBefore = (await dbq(`SELECT activation_token FROM user WHERE username = '${U}'`))?.activation_token;
f = { account: U, captcha_answer: svgCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/users/resend", f, j3);
check("重发 303 → &error=1（SMTP 失败提示，token 已轮换）", r.res.status === 303 && loc(r).includes("error=1"), loc(r));
const tokAfter = (await dbq(`SELECT activation_token FROM user WHERE username = '${U}'`))?.activation_token;
check("activation_token 已轮换且为 64 位 hex", tokAfter && tokAfter !== tokBefore && /^[0-9a-f]{64}$/.test(tokAfter), `${tokBefore} -> ${tokAfter}`);

// ── 4. 激活 ────────────────────────────────────────────────────────
r = await get(`/zh/users/activate?token=${tokAfter}`, new Jar());
check("激活页 200", r.res.status === 200, r.res.status);
const st = (await dbq(`SELECT status FROM user WHERE username = '${U}'`))?.status;
check("激活后 status=1", st === 1, String(st));

// ── 5. 激活后签入成功 ─────────────────────────────────────────────
let j5 = new Jar();
r = await get("/zh/sign-in", j5);
f = { account: U, password: P1, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, j5);
check("激活后签入 303 → /zh", r.res.status === 303 && loc(r) === "/zh", `${r.res.status} ${loc(r)}`);

// ── 6. Q2 公开资料页 ──────────────────────────────────────────────
r = await get(`/zh/users/${U}`, new Jar());
check("资料页 200 且含用户名", r.res.status === 200 && r.body.includes(U), r.res.status);
check("资料页含简介 Markdown 渲染内容", r.body.includes("About"), "intro missing");
check("资料页不泄露邮箱（匿名访问）", !r.body.includes(`${U}@example.com`), "email leaked");

// ── 7. Q6 找回密码 ────────────────────────────────────────────────
let j7 = new Jar();
r = await get("/zh/users/forgot-password", j7);
f = { account: U, captcha_answer: svgCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/users/forgot-password", f, j7);
check("找回 303 → &error=1（SMTP 失败，token 已生成）", r.res.status === 303 && loc(r).includes("error=1"), loc(r));
const rtok = await dbq(`SELECT password_reset_token, password_reset_expires_at FROM user WHERE username = '${U}'`);
check("password_reset_token 已生成", /^[0-9a-f]{64}$/.test(rtok?.password_reset_token ?? ""), String(rtok?.password_reset_token));
const now = Math.floor(Date.now() / 1000);
const exp = Number(rtok?.password_reset_expires_at);
check("重置 token 有效期约 1 小时", exp > now + 3500 && exp <= now + 3610, `exp=${exp} now=${now}`);
r = await get(`/zh/users/reset-password?token=${rtok.password_reset_token}`, j7);
check("重置页 200 含 token 隐藏域", r.res.status === 200 && r.body.includes(`value="${rtok.password_reset_token}"`), r.res.status);
f = { token: rtok.password_reset_token, new_password: P2, confirm_password: P2, csrf_token: csrf(r.body) };
r = await post("/zh/users/reset-password", f, j7);
check("重置提交 303 → sign-in?notice=reset", r.res.status === 303 && loc(r).includes("sign-in?notice=reset"), loc(r));
r = await get("/zh/account/password", j5);
check("重置后旧会话被踢（改密页显示请先登录）", r.body.includes("请先登录"), "session not kicked");
let j7b = new Jar();
r = await get("/zh/sign-in", j7b);
f = { account: U, password: P2, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, j7b);
check("新密码签入 303 → /zh", r.res.status === 303 && loc(r) === "/zh", loc(r));
let j7c = new Jar();
r = await get("/zh/sign-in", j7c);
f = { account: U, password: P1, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, j7c);
check("旧密码签入失败 error=incorrect", r.res.status === 303 && loc(r).includes("error=incorrect"), loc(r));

// ── 8. 已登录修改密码（保留当前会话） ─────────────────────────────
r = await get("/zh/account/password", j7b);
check("改密页 200（已登录）", r.res.status === 200, r.res.status);
f = { old_password: P2, new_password: P3, confirm_password: P3, csrf_token: csrf(r.body) };
r = await post("/zh/account/password", f, j7b);
check("改密 303 → ?ok=1", r.res.status === 303 && loc(r).includes("ok=1"), loc(r));
r = await get("/zh/account/password", j7b);
check("改密后当前会话保留", r.body.includes("当前密码"), r.body.includes("请先登录") ? "kicked" : "ok");

// ── 9. Q3 管理员封禁/解封 ─────────────────────────────────────────
let ja = new Jar();
r = await get("/zh/sign-in", ja);
f = { account: "admin", password: CFG.adminPass, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, ja);
check("管理员签入 303 → /zh", r.res.status === 303 && loc(r) === "/zh", loc(r));
r = await get("/zh/admin/users", ja);
check("用户管理页 200", r.res.status === 200, r.res.status);
const adminCsrf = csrf(r.body);
const rows = r.body.split('<div class="bg-surface border border-border rounded-lg p-4 flex flex-wrap items-center gap-3">').slice(1);
const idOf = {};
let adminId = "";
for (const row of rows) {
  const um = row.match(/truncate">([^<]*)</);
  const im = row.match(/user:([a-z0-9]+)\/status/);
  if (um && im) idOf[um[1]] = im[1];
  if (um && um[1] === "admin") adminId = im ? im[1] : "";
}
check("列表中解析到目标用户", !!idOf[U], JSON.stringify(idOf));
check("管理页不再显示原始键名 user_status_active", !r.body.includes(">user_status_active<"), "raw key leaked");
f = { to: "banned", csrf_token: adminCsrf };
r = await post(`/zh/admin/users/user:${idOf[U]}/status`, f, ja);
check("封禁 303 → ?ok=updated", r.res.status === 303 && loc(r).includes("ok=updated"), loc(r));
r = await get("/zh/account/password", j7b);
check("封禁后会话被踢", r.body.includes("请先登录"), "session not kicked");
let jb = new Jar();
r = await get("/zh/sign-in", jb);
f = { account: U, password: P3, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, jb);
check("封禁用户签入 303 → error=banned", r.res.status === 303 && loc(r).includes("error=banned"), loc(r));
r = await get("/zh/sign-in?error=banned", jb);
check("签入页展示封禁提示", r.body.includes("账户已被封禁"), "banned text missing");
if (adminId) {
  f = { to: "banned", csrf_token: adminCsrf };
  r = await post(`/zh/admin/users/user:${adminId}/status`, f, ja);
  check("管理员不可被封（重定向无 ok 标记）", r.res.status === 303 && !loc(r).includes("ok=updated"), loc(r));
  r = await get("/zh/admin/users", ja);
  check("管理员会话未被破坏（管理页仍 200）", r.res.status === 200, r.res.status);
}
f = { to: "active", csrf_token: adminCsrf };
r = await post(`/zh/admin/users/user:${idOf[U]}/status`, f, ja);
check("解封 303 → ?ok=updated", r.res.status === 303 && loc(r).includes("ok=updated"), loc(r));
let jc = new Jar();
r = await get("/zh/sign-in", jc);
f = { account: U, password: P3, captcha_answer: jsCap(r.body), csrf_token: csrf(r.body) };
r = await post("/zh/sign-in", f, jc);
check("解封后签入 303 → /zh", r.res.status === 303 && loc(r) === "/zh", loc(r));

// ── 10. 已登录访问本人资料页可见邮箱 ──────────────────────────────
r = await get(`/zh/users/${U}`, jc);
check("本人资料页可见邮箱", r.body.includes(`${U}@example.com`), "email hidden");


// ── 10b. 管理员任免 ─────────────────────────────────────────────
const adminRec = await dbq("SELECT id FROM user WHERE username = 'admin'");
const adminFullId = String(adminRec?.id ?? "");
const adminIdStr = adminFullId;
f = { role: "admin", csrf_token: adminCsrf };
r = await post(`/zh/admin/users/user:${idOf[U]}/role`, f, ja);
check("提升为管理员 303 → ?ok=role", r.res.status === 303 && loc(r).includes("ok=role"), loc(r));
r = await get("/zh/admin/users", jc);
check("被提升者立即获得后台权限（无需重登录）", r.res.status === 200, r.res.status);
// 管理员自己的行无任免按钮
r = await get("/zh/admin/users", ja);
const selfRows = r.body.split('<div class="bg-surface border border-border rounded-lg p-4 flex flex-wrap items-center gap-3">').slice(1);
const selfRow = selfRows.find((row) => /truncate">admin</.test(row)) ?? "";
check("管理员自己的行无任免按钮", selfRow && !selfRow.includes("/role"), "self row has role form");
// 自取消被拒
f = { role: "user", csrf_token: adminCsrf };
r = await post(`/zh/admin/users/${adminIdStr}/role`, f, ja);
check("不能取消自己的管理员（无 ok=role）", r.res.status === 303 && !loc(r).includes("ok=role"), loc(r));
r = await get("/zh/admin/users", ja);
check("自取消被拒后自己仍可访问后台", r.res.status === 200, r.res.status);
// 取消对方管理员
f = { role: "user", csrf_token: adminCsrf };
r = await post(`/zh/admin/users/user:${idOf[U]}/role`, f, ja);
check("取消管理员 303 → ?ok=role", r.res.status === 303 && loc(r).includes("ok=role"), loc(r));
r = await get("/zh/admin/users", jc);
check("被取消后立即失去后台权限（404）", r.res.status === 404, r.res.status);
// 恢复普通态后资料页正常
r = await get(`/zh/users/${U}`, new Jar());
check("取消管理员后用户资料页仍 200", r.res.status === 200, r.res.status);

console.log(failures === 0 ? "ALL PASS" : `${failures} FAILURES`);
process.exit(failures === 0 ? 0 : 1);
