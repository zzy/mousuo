// 冒烟脚本公共配置：从目标仓库 .env 读取 SurrealDB 连接（勿硬编码凭据）
// 用法：node scripts/smoke_q_mousuo.mjs 或 node scripts/smoke_q_raolv.mjs
// 前置：目标仓库服务已启动（端口 $PORT 或默认 7800）
import { Buffer } from "node:buffer";
import fs from "node:fs";

export function smokeConfig(envPath, defaultNs, defaultDb) {
  const envText = fs.readFileSync(envPath, "utf8");
  const envGet = (k) => {
    const m = envText.match(new RegExp(`^${k}=(.+)$`, "m"));
    return m ? m[1].trim() : "";
  };
  // 冒烟用管理员密码：进程 env > .env（SMOKE_ADMIN_PASS）> 当前冒烟库默认值
  const adminPass = process.env.SMOKE_ADMIN_PASS || envGet("SMOKE_ADMIN_PASS") || "Admin12345";
  let dbUrl = envGet("DB_URL") || "127.0.0.1:8000";
  // .env 中的地址可能无协议或为 ws://；REST 用 http://
  dbUrl = dbUrl.replace(/^ws:\/\//, "http://").replace(/^wss:\/\//, "https://");
  if (!/^https?:\/\//.test(dbUrl)) dbUrl = "http://" + dbUrl;
  return {
    base: `http://127.0.0.1:${process.env.PORT || "7800"}`,
    db: `${dbUrl}/sql`,
    dbAuth:
      "Basic " +
      Buffer.from(`${envGet("DB_USER") || "root"}:${envGet("DB_PASS")}`).toString("base64"),
    ns: envGet("DB_NS") || defaultNs,
    dbName: envGet("DB_NAME") || defaultDb,
    adminPass,
  };
}
