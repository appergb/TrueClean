## 变更说明

<!-- 简要描述本 PR 做了什么、为什么 -->

## 变更类型

- [ ] 新功能 (feat)
- [ ] Bug 修复 (fix)
- [ ] 重构 (refactor)
- [ ] 文档 (docs)
- [ ] CI/CD (chore)
- [ ] 测试 (test)

## 契约合规

- [ ] 未修改冻结文件（model.rs / error.rs / state.rs / lib.rs / main.rs / mod.rs / commands/settings.rs / src/lib/{types,ipc,format}.ts / src/main.tsx / src/styles/{tokens,global}.css）
- [ ] Tauri 命令名、事件名逐字一致
- [ ] 数据模型字段一一对应（camelCase）

## 验收门禁

- [ ] `pnpm lint` 通过（0 errors）
- [ ] `pnpm build` 通过（tsc + vite）
- [ ] `cd src-tauri && cargo fmt --all -- --check` 通过
- [ ] `cd src-tauri && cargo clippy --all-targets -- -D warnings` 通过
- [ ] `cd src-tauri && cargo test` 通过
- [ ] 新增功能有对应测试或手动验证说明

## 补充说明

<!-- 截图、性能数据、已知问题等 -->
