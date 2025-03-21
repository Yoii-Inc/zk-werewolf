-- Supabaseにユーザーテーブルを作成するSQL
-- このスクリプトはSupabaseコンソールのSQL Editorで実行してください

-- ユーザーテーブル
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY,
  username TEXT NOT NULL,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
  last_login TIMESTAMP WITH TIME ZONE
);

-- RLSポリシーの設定
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

-- 匿名ユーザーが新規ユーザーを作成できるようにする
CREATE POLICY "ユーザー登録を許可" ON users
  FOR INSERT TO anon
  WITH CHECK (true);

-- ユーザー自身のデータのみを取得・更新できるようにする
CREATE POLICY "ユーザーは自分のデータのみ閲覧可能" ON users
  FOR SELECT TO authenticated
  USING (auth.uid() = id);

CREATE POLICY "ユーザーは自分のデータのみ更新可能" ON users
  FOR UPDATE TO authenticated
  USING (auth.uid() = id);

-- メールアドレスによるユーザー検索を許可
CREATE POLICY "メールアドレスによるユーザー検索を許可" ON users
  FOR SELECT TO anon
  USING (true);

-- インデックス作成
CREATE INDEX IF NOT EXISTS users_email_idx ON users (email);