# aio-plugin-studio / AIO Studio Plugin

Studio 的独立功能仓库，保存从原 AIO 平台提取的完整编辑器、定义模型、编译器、生成器、运行逻辑、数据库迁移和人工业务实现。Git 历史按所属路径过滤保留。

The standalone feature repository for Studio, holding the full editor, definition models, compiler, generator, runtime logic, database migrations and human-owned business implementation extracted from the original AIO platform. Git history is preserved, filtered by owning paths.

## 当前布局 / Current Layout

- `app/plugins/studio`：Studio 编辑器、模型、编译与页面解释器。
- `app`：迁移中的 Studio 独立启动入口与正式数据库迁移。
- `lib/biz`：生成业务契约和已由人工接管的实现。
- `generated/apps`：已有源码导出结果。
- `scripts`：Studio 本地预览与数据库连接辅助。

- `app/plugins/studio`: Studio editor, models, compilation and page interpreter.
- `app`: the Studio standalone entry point being migrated, plus the production database migrations.
- `lib/biz`: generated business contracts and implementations already taken over by humans.
- `generated/apps`: results of existing source exports.
- `scripts`: Studio local preview and database connection helpers.

这些代码不再属于平台 workspace。基础运行库固定消费 `aio-platform` 的 `az-plugin-core`，UI 固定消费独立 workbench crates。

These crates no longer belong to the platform workspace. Base runtime libraries consume `az-plugin-core` from `aio-platform`; the UI consumes the standalone workbench crates.

## 验证 / Verification

```sh
cargo check -p az-studio -p az-aio-app
```

本次提取没有修改业务实现和正式数据库，不删除账户或业务数据。正式连接配置需要由运行环境提供，不能用测试库替换生产配置。

This extraction did not modify business implementations or the production database, and does not delete accounts or business data. Production connection configuration must come from the runtime environment; a test database cannot replace production configuration.

## 待迁移 / Migration Roadmap

当前完成的是仓库归属拆分，不是 Studio 在线替换。接下来将前后端收敛到 `frontend/`、`backend/`、`shared/` 并接入 `aio:plugin@2.0.0`；解释器保留在本插件，源码导出改为独立全栈插件工程，不能再生成平台内部模块或安装时编译平台。

What is done so far is a repository ownership split, not an in-place Studio replacement. Next, the frontend and backend will converge into `frontend/`, `backend/` and `shared/`, and plug into `aio:plugin@2.0.0`; the interpreter stays in this plugin, source export becomes an independent full-stack plugin project, and the platform can no longer generate internal modules or compile the platform at install time.
