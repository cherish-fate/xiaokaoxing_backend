-- 1. 创建校考星数据库（不存在才创建）
CREATE DATABASE IF NOT EXISTS xkx_background DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- 2. 选中数据库（关键！解决报错）
USE xkx_background;

-- 3. 创建用户表
CREATE TABLE IF NOT EXISTS users (
                                     id            INT AUTO_INCREMENT PRIMARY KEY,
                                     email         VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    nickname      VARCHAR(50),
    school_id     INT,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP
    );