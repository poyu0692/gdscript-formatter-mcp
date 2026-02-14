# gdscript-formatter-mcp 開発ガイド

このドキュメントは AI エージェント（Claude Code など）がこのプロジェクトで作業する際の指示を記載しています。

## プロジェクト概要

GDQuest の `GDScript-formatter` を MCP サーバーとしてラップし、AI が GDScript のフォーマット・Lint を直接実行できるようにする Rust 製ツールです。

## コーディング規約

### Rust コードの品質管理

コードを編集・追加したら、**必ず以下を実行してください**:

#### 1. フォーマットチェック

```bash
cargo fmt --check
```

問題があれば自動修正:

```bash
cargo fmt
```

#### 2. Clippy による Lint

```bash
cargo clippy -- -D warnings
```

警告が出た場合は修正してください。

#### 3. テスト実行

```bash
cargo test
```

全てのテストが通ることを確認してください。

#### 4. ビルド確認

```bash
cargo build --release
```

リリースビルドが成功することを確認してください。

## 典型的なワークフロー

### 新機能追加時

1. 該当ファイルを編集（例: `src/tools/format.rs`）
2. `cargo fmt` でフォーマット
3. `cargo clippy` で Lint チェック
4. `cargo test` でテスト実行
5. 必要に応じて新しいテストを追加
6. `cargo build --release` で最終確認

### バグ修正時

1. 問題を再現するテストを追加
2. コードを修正
3. `cargo test` でテストが通ることを確認
4. `cargo clippy` で Lint チェック
5. `cargo fmt` でフォーマット

### ドキュメント更新時

- `README.md` を編集したら、内容が矛盾していないか確認
- インストール手順が実際に動作するか確認
- 例示されているコマンドが正しいか確認

## リリースプロセス

新しいバージョンをリリースする場合:

1. `Cargo.toml` のバージョンを更新
2. `CHANGELOG.md` を更新（もしあれば）
3. 全てのテストとビルドを確認
4. タグを作成してプッシュ:
   ```bash
   git tag v0.x.x
   git push origin v0.x.x
   ```
5. GitHub Actions が自動的に全プラットフォーム向けのバイナリをビルド・リリース

## 注意事項

### セキュリティ

- ユーザー入力（ファイルパス、引数など）は必ず検証する
- コマンドインジェクションのリスクを考慮する
- 環境変数の扱いに注意する

### 互換性

- 既存の MCP クライアントとの互換性を保つ
- `structuredContent` のフォーマットを破壊的に変更しない（バージョンアップ時のみ）
- GDScript-formatter の新しいバージョンにも対応できるようにする

### テスト

- 新機能には必ずテストを追加
- エッジケース（空配列、存在しないファイルなど）をテスト
- クロスプラットフォームでの動作を考慮

## 参考情報

- MCP プロトコル仕様: https://modelcontextprotocol.io/
- GDScript-formatter: https://github.com/GDQuest/GDScript-formatter
- このプロジェクトの README: [README.md](README.md)
