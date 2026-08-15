-- 管理端升级脚本：为已有数据库补充管理员字段、考点拒绝原因和默认管理员账号。
USE xkx_background;

SET @sql_add_is_admin = IF(
    (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'users' AND COLUMN_NAME = 'is_admin') = 0,
    "ALTER TABLE `users` ADD COLUMN `is_admin` tinyint(1) NOT NULL DEFAULT '0' COMMENT '是否管理员（0-否，1-是）' AFTER `major_id`",
    'SELECT 1'
);
PREPARE stmt_add_is_admin FROM @sql_add_is_admin;
EXECUTE stmt_add_is_admin;
DEALLOCATE PREPARE stmt_add_is_admin;

SET @sql_add_is_disabled = IF(
    (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'users' AND COLUMN_NAME = 'is_disabled') = 0,
    "ALTER TABLE `users` ADD COLUMN `is_disabled` tinyint(1) NOT NULL DEFAULT '0' COMMENT '是否禁用（0-否，1-是）' AFTER `is_admin`",
    'SELECT 1'
);
PREPARE stmt_add_is_disabled FROM @sql_add_is_disabled;
EXECUTE stmt_add_is_disabled;
DEALLOCATE PREPARE stmt_add_is_disabled;

SET @sql_add_vote_reason = IF(
    (SELECT COUNT(*) FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'votes' AND COLUMN_NAME = 'reject_reason') = 0,
    "ALTER TABLE `votes` ADD COLUMN `reject_reason` varchar(500) DEFAULT NULL COMMENT '审核拒绝原因' AFTER `status`",
    'SELECT 1'
);
PREPARE stmt_add_vote_reason FROM @sql_add_vote_reason;
EXECUTE stmt_add_vote_reason;
DEALLOCATE PREPARE stmt_add_vote_reason;

-- 默认管理员：admin@xiaokaoxing.com / admin123
INSERT IGNORE INTO users (nickname, email, password_hash, school_name, major_id, avatar_url, is_admin, is_disabled)
VALUES ('系统管理员', 'admin@xiaokaoxing.com', '$argon2id$v=19$m=19456,t=2,p=1$OdIhZFSgfi2tQ+SypC8WNg$SEAlwytIkkq3I7HQhWbHYRT7rroqeZzzF8bUM8oIN3U', '校考星', 1, NULL, 1, 0);