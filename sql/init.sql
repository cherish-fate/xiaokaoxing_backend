-- 1. 创建校考星数据库（不存在才创建）
CREATE DATABASE IF NOT EXISTS xkx_background DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- 2. 选中数据库
USE xkx_background;

-- 4. 专业表（无学校关联）
CREATE TABLE `majors` (
                          `id` int NOT NULL AUTO_INCREMENT COMMENT '专业ID，主键',
                          `name` varchar(100) NOT NULL COMMENT '专业名称（例如：计算机科学与技术）',
                          PRIMARY KEY (`id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='专业信息表';

-- 5. 用户表（直接存储学校名称，外键关联专业）
CREATE TABLE `users` (
                         `id` int NOT NULL AUTO_INCREMENT COMMENT '用户ID，主键',
                         `nickname` varchar(50) NOT NULL COMMENT '用户昵称',
                         `email` varchar(100) NOT NULL COMMENT '用户邮箱（登录账号）',
                         `password_hash` varchar(255) NOT NULL COMMENT '加密后的密码',
                         `school_name` varchar(100) NOT NULL COMMENT '用户所在学校名称（直接存储文本）',
                         `major_id` int NOT NULL COMMENT '用户所学专业ID（外键关联 majors.id）',
                         `avatar_url` varchar(500) DEFAULT NULL COMMENT '头像图片URL（可选）',
                         `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '注册时间',
                         `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '信息最后更新时间',
                         PRIMARY KEY (`id`),
                         UNIQUE KEY `uk_email` (`email`) COMMENT '邮箱唯一，保证注册不重复',
                         KEY `idx_school_name` (`school_name`) COMMENT '为学校名称加索引，便于按学校查询统计',
                         CONSTRAINT `fk_user_major` FOREIGN KEY (`major_id`) REFERENCES `majors` (`id`) ON DELETE RESTRICT ON UPDATE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户核心信息表';


INSERT INTO `majors` (`name`) VALUES
                                  ('计算机科学与技术'),
                                  ('软件工程'),
                                  ('网络工程'),
                                  ('信息安全'),
                                  ('物联网工程'),
                                  ('数据科学与大数据技术'),
                                  ('人工智能'),
                                  ('电子信息工程'),
                                  ('通信工程'),
                                  ('自动化'),
                                  ('机器人工程'),
                                  ('机械设计制造及其自动化'),
                                  ('车辆工程'),
                                  ('材料科学与工程'),
                                  ('能源与动力工程'),
                                  ('电气工程及其自动化'),
                                  ('土木工程'),
                                  ('建筑环境与能源应用工程'),
                                  ('水利水电工程'),
                                  ('测绘工程'),
                                  ('化学工程与工艺'),
                                  ('制药工程'),
                                  ('环境工程'),
                                  ('生物医学工程'),
                                  ('食品科学与工程'),
                                  ('建筑学'),
                                  ('城乡规划'),
                                  ('风景园林'),
                                  ('数学与应用数学'),
                                  ('物理学'),
                                  ('化学'),
                                  ('生物科学'),
                                  ('生物技术'),
                                  ('生态学'),
                                  ('统计学'),
                                  ('应用统计学'),
                                  ('经济学'),
                                  ('金融学'),
                                  ('金融工程'),
                                  ('国际经济与贸易'),
                                  ('财政学'),
                                  ('税收学'),
                                  ('工商管理'),
                                  ('市场营销'),
                                  ('会计学'),
                                  ('财务管理'),
                                  ('人力资源管理'),
                                  ('审计学'),
                                  ('行政管理'),
                                  ('公共事业管理'),
                                  ('物流管理'),
                                  ('电子商务'),
                                  ('旅游管理'),
                                  ('酒店管理'),
                                  ('法学'),
                                  ('政治学与行政学'),
                                  ('社会学'),
                                  ('社会工作'),
                                  ('汉语言文学'),
                                  ('英语'),
                                  ('日语'),
                                  ('翻译'),
                                  ('新闻学'),
                                  ('广告学'),
                                  ('传播学'),
                                  ('教育学'),
                                  ('教育技术学'),
                                  ('学前教育'),
                                  ('小学教育'),
                                  ('体育教育'),
                                  ('历史学'),
                                  ('考古学'),
                                  ('哲学'),
                                  ('艺术史论'),
                                  ('音乐表演'),
                                  ('舞蹈表演'),
                                  ('戏剧影视文学'),
                                  ('广播电视编导'),
                                  ('播音与主持艺术'),
                                  ('美术学'),
                                  ('视觉传达设计'),
                                  ('环境设计'),
                                  ('产品设计'),
                                  ('服装与服饰设计'),
                                  ('数字媒体艺术'),
                                  ('农学'),
                                  ('园艺'),
                                  ('植物保护'),
                                  ('动物科学'),
                                  ('动物医学'),
                                  ('林学'),
                                  ('园林'),
                                  ('水产养殖学'),
                                  ('临床医学'),
                                  ('麻醉学'),
                                  ('医学影像学'),
                                  ('口腔医学'),
                                  ('预防医学'),
                                  ('中医学'),
                                  ('针灸推拿学'),
                                  ('药学'),
                                  ('药物制剂'),
                                  ('中药学'),
                                  ('医学检验技术'),
                                  ('康复治疗学'),
                                  ('护理学');

INSERT INTO `users` (
    `nickname`,
    `email`,
    `password_hash`,
    `school_name`,
    `major_id`,
    `avatar_url`
) VALUES
      (
          '张三',
          'zhangsan@test.com',
          '$2y$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy',  -- 示例哈希（明文 password123）
          '南阳理工学院',
          1,  -- 计算机科学与技术
          NULL
      ),
      (
          '李四',
          'lisi@test.com',
          '$2y$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy',
          '郑州大学',
          2,  -- 软件工程
          'https://example.com/avatars/lisi.jpg'
      ),
      (
          '王五',
          'wangwu@test.com',
          '$2y$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy',
          '河南大学',
          3,  -- 网络工程
          NULL
      ),
      (
          '赵六',
          'zhaoliu@test.com',
          '$2y$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy',
          '南阳师范学院',
          4,  -- 电子信息工程
          'https://example.com/avatars/zhaoliu.png'
      ),
      (
          '孙七',
          'sunqi@test.com',
          '$2y$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy',
          '北京大学',
          5,  -- 通信工程
          NULL
      );

-- 6. 考试信息表
CREATE TABLE `exams` (
    `id` int NOT NULL AUTO_INCREMENT COMMENT '考试ID，主键',
    `user_id` int NOT NULL COMMENT '所属用户ID，逻辑关联users表',
    `name` varchar(100) NOT NULL COMMENT '考试名称（如：高等数学）',
    `exam_date` date NOT NULL COMMENT '考试日期',
    `start_time` time NOT NULL COMMENT '考试开始时间（如：09:00）',
    `end_time` time DEFAULT NULL COMMENT '考试结束时间（如：10:00），可选',
    `location` varchar(200) DEFAULT NULL COMMENT '考试地点（如：综合楼201）',
    `is_completed` tinyint(1) DEFAULT '0' COMMENT '是否已完成（0-待完成，1-已完成），用于统计今日进度',
    `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
    PRIMARY KEY (`id`),
    KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引，便于查询该用户所有考试',
    KEY `idx_exam_date` (`exam_date`) COMMENT '考试日期索引，便于排序和倒计时计算'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='考试信息表';

-- 7. 学习任务/计划表
CREATE TABLE `tasks` (
    `id` int NOT NULL AUTO_INCREMENT COMMENT '任务ID，主键',
    `user_id` int NOT NULL COMMENT '所属用户ID，逻辑关联users表',
    `task_name` varchar(200) NOT NULL COMMENT '任务名称（如：复习高等数学第三章）',
    `plan_date` date NOT NULL COMMENT '计划执行日期（用于筛选今日任务）',
    `is_completed` tinyint(1) DEFAULT '0' COMMENT '是否已完成（0-待完成，1-已完成）',
    `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
    PRIMARY KEY (`id`),
    KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
    KEY `idx_plan_date` (`plan_date`) COMMENT '计划日期索引，用于快速查询今日任务'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='学习任务/计划表';

-- 8. 备考资源表
CREATE TABLE `resources` (
    `id` int NOT NULL AUTO_INCREMENT COMMENT '资源ID，主键',
    `title` varchar(200) NOT NULL COMMENT '资源标题（如：高数期末真题汇总（2020-2024））',
    `type_tag` varchar(50) NOT NULL COMMENT '资源类型标签（如：本校专属、高频必考）',
    `school_name` varchar(100) DEFAULT NULL COMMENT '所属学校（NULL表示通用资源，全校可用）',
    `major_id` int DEFAULT NULL COMMENT '关联专业ID（NULL表示通用资源，所有专业可用），逻辑关联majors表',
    `description` text COMMENT '资源描述/简介（如：包含近5年高数期末真题及解析）',
    `file_url` varchar(500) NOT NULL COMMENT '资源文件URL或外部链接',
    `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
    PRIMARY KEY (`id`),
    KEY `idx_school_name` (`school_name`) COMMENT '学校索引，用于筛选本校资源',
    KEY `idx_major_id` (`major_id`) COMMENT '专业索引，用于筛选相关专业资源',
    KEY `idx_type_tag` (`type_tag`) COMMENT '类型标签索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='备考资源表';

-- 示例数据：考试
INSERT INTO `exams` (`user_id`, `name`, `exam_date`, `start_time`, `end_time`, `location`, `is_completed`) VALUES
    (1, '高等数学', '2026-08-21', '09:00:00', '10:00:00', '综合楼201', 0),
    (1, '大学英语', '2026-08-25', '14:00:00', '16:00:00', '综合楼301', 0),
    (1, '线性代数', '2026-09-10', '09:00:00', '11:00:00', '教学楼A102', 0),
    (1, '计算机组成原理', '2026-07-15', '09:00:00', '10:30:00', '实验楼B201', 1),
    (2, '软件工程', '2026-08-30', '14:00:00', '16:00:00', '教学楼C305', 0),
    (2, '数据库原理', '2026-09-05', '09:00:00', '11:00:00', '实验楼B101', 0);

-- 示例数据：任务（今日任务基于当前日期动态生成，这里只做示例）
INSERT INTO `tasks` (`user_id`, `task_name`, `plan_date`, `is_completed`) VALUES
    (1, '复习高等数学第三章', '2026-08-06', 1),
    (1, '完成英语四级真题一套', '2026-08-06', 0),
    (1, '整理计算机组成原理笔记', '2026-08-06', 0),
    (1, '做10道线性代数习题', '2026-08-07', 0),
    (2, '复习软件工程需求分析', '2026-08-06', 0);

-- 示例数据：资源
INSERT INTO `resources` (`title`, `type_tag`, `school_name`, `major_id`, `description`, `file_url`) VALUES
    ('高数期末真题汇总（2020-2024）', '本校专属', '南阳理工学院', 1, '包含近5年高数期末真题及解析', 'https://example.com/resource/1'),
    ('数据结构期末复习资料', '本校专属', '南阳理工学院', 1, '数据结构重点知识点梳理', 'https://example.com/resource/3'),
    ('英语四六级重点词汇', '高频必考', NULL, 2, '四六级高频词汇汇总', 'https://example.com/resource/2'),
    ('考研数学基础知识点', '高频必考', NULL, NULL, '考研数学核心考点汇总', 'https://example.com/resource/4'),
    ('软件工程导论笔记', '本校专属', '郑州大学', 2, '软件工程重点概念整理', 'https://example.com/resource/5'),
    ('计算机网络期末真题', '本校专属', '南阳理工学院', 1, '近3年计算机网络期末真题', 'https://example.com/resource/6');

-- 添加分类字段
ALTER TABLE `resources` ADD COLUMN `category` varchar(50) NOT NULL DEFAULT '真题试卷' COMMENT '资源分类（真题试卷、复习提纲、课件考点、自测题库）' AFTER `title`;

-- 添加作者字段
ALTER TABLE `resources` ADD COLUMN `author` varchar(50) DEFAULT NULL COMMENT '来源作者（如：本校学长、本校老师）' AFTER `major_id`;

-- 添加浏览量字段
ALTER TABLE `resources` ADD COLUMN `view_count` int DEFAULT '0' COMMENT '浏览量' AFTER `file_url`;

-- 添加热门标识字段
ALTER TABLE `resources` ADD COLUMN `is_hot` tinyint(1) DEFAULT '0' COMMENT '是否热门（0-否，1-是）' AFTER `view_count`;

-- 添加索引
ALTER TABLE `resources` ADD INDEX `idx_category` (`category`);
ALTER TABLE `resources` ADD INDEX `idx_view_count` (`view_count`);

-- ============================================
-- 1. 收藏表（favorites）
-- ============================================
CREATE TABLE `favorites` (
                             `id` int NOT NULL AUTO_INCREMENT COMMENT '收藏记录ID，主键',
                             `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                             `resource_id` int NOT NULL COMMENT '资源ID，逻辑关联resources表',
                             `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '收藏时间',
                             PRIMARY KEY (`id`),
                             UNIQUE KEY `uk_user_resource` (`user_id`, `resource_id`) COMMENT '唯一约束，防止同一用户重复收藏同一资源',
                             KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引，查询用户所有收藏',
                             KEY `idx_resource_id` (`resource_id`) COMMENT '资源ID索引，查询资源被收藏次数'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户收藏表';

-- ============================================
-- 2. AI对话记录表（ai_conversations）
-- ============================================
CREATE TABLE `ai_conversations` (
                                    `id` int NOT NULL AUTO_INCREMENT COMMENT '消息记录ID，主键',
                                    `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                                    `conversation_id` varchar(50) NOT NULL COMMENT '会话ID，同一会话共享相同ID，用于多轮对话分组',
                                    `role` varchar(20) NOT NULL COMMENT '角色：user-用户消息，assistant-AI回复',
                                    `content` text NOT NULL COMMENT '消息内容',
                                    `attachment_url` varchar(500) DEFAULT NULL COMMENT '附件URL（用户上传的文件，可选）',
                                    `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                                    PRIMARY KEY (`id`),
                                    KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引，查询用户所有对话记录',
                                    KEY `idx_conversation_id` (`conversation_id`) COMMENT '会话ID索引，查询某次会话的所有消息',
                                    KEY `idx_user_conversation` (`user_id`, `conversation_id`) COMMENT '联合索引，查询用户某次会话历史',
                                    KEY `idx_created_at` (`created_at`) COMMENT '创建时间索引，按时间排序'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='AI对话记录表';

-- 插入资源测试数据
INSERT INTO `resources` (
    `title`,
    `category`,
    `type_tag`,
    `school_name`,
    `major_id`,
    `author`,
    `description`,
    `file_url`,
    `view_count`,
    `is_hot`
) VALUES
      (
          '2024高数期末真题A卷',
          '真题试卷',
          '本校专属',
          '南阳理工学院',
          1,  -- 计算机科学与技术
          '本校学长',
          '2024年高等数学期末考试真题A卷，含参考答案和评分标准，适用于期末冲刺复习。',
          'https://example.com/resources/2024_gaoshu_zhenti_A.pdf',
          1203,
          1
      ),
      (
          '高数第1-5章复习提纲',
          '复习提纲',
          '本校专属',
          '南阳理工学院',
          1,  -- 计算机科学与技术
          '本校老师',
          '高等数学第1-5章重点知识点梳理，包含极限、导数、积分等核心考点，附带典型例题。',
          'https://example.com/resources/gaoshu_fuxi_outline.pdf',
          876,
          0
      ),
      (
          '极限·导数·积分考点汇编',
          '课件考点',
          '本校专属',
          '南阳理工学院',
          1,  -- 计算机科学与技术
          '本校学长',
          '高数核心考点汇编，包含极限的七种求法、导数应用、积分计算方法等，适合考前快速回顾。',
          'https://example.com/resources/gaoshu_kaodian_huibian.pdf',
          654,
          0
      ),
      (
          '英语四六级重点词汇',
          '自测题库',
          '高频必考',
          NULL,  -- 通用资源
          2,  -- 软件工程
          '系统整理',
          '大学英语四六级高频词汇汇总，包含核心词汇、常考短语、搭配用法，附带自测练习。',
          'https://example.com/resources/cet4_6_vocabulary.pdf',
          2301,
          1
      ),
      (
          '软件工程导论期末复习笔记',
          '复习提纲',
          '本校专属',
          '郑州大学',
          2,  -- 软件工程
          '本校学长',
          '软件工程导论重点概念整理，包含软件开发模型、需求分析、软件测试等核心知识点。',
          'https://example.com/resources/software_engineering_notes.pdf',
          543,
          0
      );

-- 插入收藏测试数据
INSERT INTO `favorites` (
    `user_id`,
    `resource_id`
) VALUES
-- 用户1（张三）的收藏
(
    1,  -- user_id: 张三
    1   -- resource_id: 2024高数期末真题A卷
),
(
    1,  -- user_id: 张三
    4   -- resource_id: 英语四六级重点词汇
),
(
    1,  -- user_id: 张三
    5   -- resource_id: 软件工程导论期末复习笔记
),
-- 用户2（李四）的收藏
(
    2,  -- user_id: 李四
    1   -- resource_id: 2024高数期末真题A卷
),
(
    2,  -- user_id: 李四
    2   -- resource_id: 高数第1-5章复习提纲
);

-- 添加审核状态字段（直接存储中文状态）
ALTER TABLE `resources`
    ADD COLUMN `status` varchar(10) DEFAULT '审核中' COMMENT '审核状态（审核中/已上线/未通过）' AFTER `file_url`;

-- 添加索引
ALTER TABLE `resources` ADD INDEX `idx_status` (`status`);

CREATE TABLE `checkin_records` (
                                   `id` int NOT NULL AUTO_INCREMENT COMMENT '打卡记录ID，主键',
                                   `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                                   `checkin_date` date NOT NULL COMMENT '打卡日期',
                                   `subjects` json DEFAULT NULL COMMENT '学习科目列表，JSON数组（如：["高等数学","大学英语"]）',
                                   `duration` varchar(10) DEFAULT NULL COMMENT '学习时长（30min/1h/2h/3h+）',
                                   `note` varchar(100) DEFAULT NULL COMMENT '备注，最多100字',
                                   `tags` json DEFAULT NULL COMMENT '快速标签，JSON数组（如：["💪 有进步","🎯 达成目标"]）',
                                   `continuous_days` int DEFAULT '0' COMMENT '打卡时的连续天数（后端计算）',
                                   `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                                   `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                                   PRIMARY KEY (`id`),
                                   UNIQUE KEY `uk_user_date` (`user_id`, `checkin_date`) COMMENT '同一用户同一天只能打卡一次',
                                   KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                                   KEY `idx_checkin_date` (`checkin_date`) COMMENT '打卡日期索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='打卡记录表';

CREATE TABLE `teams` (
                         `id` int NOT NULL AUTO_INCREMENT COMMENT '小队ID，主键',
                         `name` varchar(20) NOT NULL COMMENT '小队名称，最多20字',
                         `subject` varchar(50) NOT NULL COMMENT '关联科目',
                         `description` varchar(100) DEFAULT NULL COMMENT '小队简介，最多100字',
                         `creator_id` int NOT NULL COMMENT '创建者用户ID，逻辑关联users表',
                         `member_count` int DEFAULT '1' COMMENT '当前成员数（含创建者）',
                         `max_members` int DEFAULT '30' COMMENT '成员上限，固定30',
                         `need_approval` tinyint(1) DEFAULT '1' COMMENT '是否需要审核加入（1-需要，0-不需要）',
                         `checkin_rate` decimal(5,2) DEFAULT '0.00' COMMENT '今日打卡率（百分比，冗余字段）',
                         `total_checkins` int DEFAULT '0' COMMENT '总打卡次数（冗余字段）',
                         `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                         `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                         PRIMARY KEY (`id`),
                         KEY `idx_subject` (`subject`) COMMENT '科目索引',
                         KEY `idx_creator_id` (`creator_id`) COMMENT '创建者索引',
                         KEY `idx_checkin_rate` (`checkin_rate`) COMMENT '打卡率索引，用于热门排序'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='备考小队表';

CREATE TABLE `team_members` (
                                `id` int NOT NULL AUTO_INCREMENT COMMENT '成员记录ID，主键',
                                `team_id` int NOT NULL COMMENT '小队ID，逻辑关联teams表',
                                `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                                `role` varchar(10) NOT NULL DEFAULT '成员' COMMENT '角色（队长/成员）',
                                `joined_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '加入时间',
                                PRIMARY KEY (`id`),
                                UNIQUE KEY `uk_team_user` (`team_id`, `user_id`) COMMENT '唯一约束，防止重复加入',
                                KEY `idx_team_id` (`team_id`) COMMENT '小队ID索引',
                                KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='小队成员表';

CREATE TABLE `team_join_requests` (
                                      `id` int NOT NULL AUTO_INCREMENT COMMENT '申请记录ID，主键',
                                      `team_id` int NOT NULL COMMENT '小队ID，逻辑关联teams表',
                                      `user_id` int NOT NULL COMMENT '申请用户ID，逻辑关联users表',
                                      `status` varchar(10) NOT NULL DEFAULT '待审核' COMMENT '申请状态（待审核/已通过/已拒绝）',
                                      `applied_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '申请时间',
                                      `processed_at` datetime DEFAULT NULL COMMENT '处理时间（审核通过/拒绝的时间）',
                                      PRIMARY KEY (`id`),
                                      UNIQUE KEY `uk_team_user` (`team_id`, `user_id`) COMMENT '同一用户对小队的申请唯一，防止重复申请',
                                      KEY `idx_team_id` (`team_id`) COMMENT '小队ID索引',
                                      KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                                      KEY `idx_status` (`status`) COMMENT '状态索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='入队申请表';

CREATE TABLE `votes` (
                         `id` int NOT NULL AUTO_INCREMENT COMMENT '投票ID，主键',
                         `subject` varchar(50) NOT NULL COMMENT '科目',
                         `title` varchar(30) NOT NULL COMMENT '考点名称，最多30字',
                         `description` varchar(200) DEFAULT NULL COMMENT '补充说明，最多200字',
                         `vote_count` int DEFAULT '0' COMMENT '总票数（冗余字段）',
                         `confidence` decimal(5,2) DEFAULT '0.00' COMMENT '置信度（百分比，冗余字段）',
                         `status` varchar(10) NOT NULL DEFAULT '待审核' COMMENT '审核状态（待审核/已通过/已拒绝）',
                         `submitter_id` int NOT NULL COMMENT '提交者用户ID，逻辑关联users表',
                         `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                         `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                         PRIMARY KEY (`id`),
                         KEY `idx_subject` (`subject`) COMMENT '科目索引',
                         KEY `idx_status` (`status`) COMMENT '状态索引',
                         KEY `idx_vote_count` (`vote_count`) COMMENT '票数索引，用于热门排序'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='考点投票表';

CREATE TABLE `vote_records` (
                                `id` int NOT NULL AUTO_INCREMENT COMMENT '投票记录ID，主键',
                                `vote_id` int NOT NULL COMMENT '投票ID，逻辑关联votes表',
                                `user_id` int NOT NULL COMMENT '投票用户ID，逻辑关联users表',
                                `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '投票时间',
                                PRIMARY KEY (`id`),
                                UNIQUE KEY `uk_vote_user` (`vote_id`, `user_id`) COMMENT '同一用户对同一考点只能投一票',
                                KEY `idx_vote_id` (`vote_id`) COMMENT '投票ID索引',
                                KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='投票记录表';

-- 5个小队，其中3个创建者为user_id=7
INSERT INTO teams (name, subject, description, creator_id, member_count, need_approval, checkin_rate, total_checkins)
VALUES
    ('高数突击队', '高等数学', '一起攻克高数难关', 7, 1, 1, 0.00, 0),
    ('英语打卡营', '大学英语', '每日背单词打卡', 7, 1, 1, 0.00, 0),
    ('计科期末冲刺', '计算机', '计科专业备考互助', 7, 1, 0, 0.00, 0),
    ('线代过关', '线性代数', '线代不挂科', 1, 1, 1, 0.00, 0),
    ('大物答疑', '大学物理', '物理难题讨论', 2, 1, 1, 0.00, 0);

-- 5个投票项目，其中3个提交者为user_id=7，并设置为已通过
INSERT INTO votes (subject, title, description, vote_count, confidence, status, submitter_id)
VALUES
    ('高数', '极限与连续', '常考大题，年年必出', 10, 85.00, '已通过', 7),
    ('线代', '矩阵运算', '基础运算，必须掌握', 8, 72.00, '已通过', 7),
    ('数据结构', '二叉树遍历', '重点算法，频繁考察', 6, 60.00, '已通过', 7),
    ('英语', '阅读理解', '高频题型，需强化', 0, 0.00, '待审核', 1),
    ('物理', '电磁感应', '难点，易出大题', 0, 0.00, '待审核', 2);

-- 5条打卡记录，4条user_id=7，1条user_id=1
INSERT INTO checkin_records (user_id, checkin_date, subjects, duration, note, tags, continuous_days)
VALUES
    (7, '2026-08-01', '["高等数学"]', '1h', '复习了极限与连续', '["💪 有进步"]', 1),
    (7, '2026-08-02', '["高等数学","大学英语"]', '2h', '做了高数题和英语阅读', '["🎯 达成目标"]', 2),
    (7, '2026-08-03', '["线性代数"]', '30min', '预习了矩阵运算', '["❓ 有疑问"]', 3),
    (7, '2026-08-04', '["数据结构"]', '3h+', '完成了二叉树遍历练习', '["💪 有进步","🎯 达成目标"]', 4),
    (1, '2026-08-05', '["高等数学"]', '1h', '复习高数第一章', '["💪 有进步"]', 1);

-- 5条成员记录，3条user_id=7（作为队长），2条其他用户加入
-- 注意：team_id 对应上一步插入的 teams 的 id（此处假设自增后 id 为 1~5）
INSERT INTO team_members (team_id, user_id, role)
VALUES
    (1, 7, '队长'),   -- 高数突击队 队长
    (2, 7, '队长'),   -- 英语打卡营 队长
    (3, 7, '队长'),   -- 计科期末冲刺 队长
    (1, 1, '成员'),   -- 用户1加入高数突击队
    (2, 2, '成员');   -- 用户2加入英语打卡营

-- 5条申请记录，3条user_id=7，2条其他用户
-- 关联的 team_id 同样为 1~5
INSERT INTO team_join_requests (team_id, user_id, status, processed_at)
VALUES
    (4, 7, '待审核', NULL),                          -- 申请加入线代过关
    (5, 7, '已通过', NOW()),                        -- 申请加入大物答疑
    (1, 7, '已拒绝', NOW()),                        -- 申请加入自己创建的小队（被拒）
    (1, 3, '待审核', NULL),                         -- 用户3申请高数突击队
    (2, 4, '已通过', NOW());                        -- 用户4申请英语打卡营

-- 5条投票记录，3条user_id=7，2条其他用户
-- 关联的 vote_id 对应 votes 的 id（假设自增后为 1~5）
INSERT INTO vote_records (vote_id, user_id)
VALUES
    (1, 7),   -- 用户7投高数-极限与连续
    (2, 7),   -- 用户7投线代-矩阵运算
    (4, 7),   -- 用户7投英语-阅读理解（虽然该投票待审核，但仍可投，实际业务中只允许投已通过的，此处测试）
    (1, 1),   -- 用户1投高数-极限与连续
    (2, 2);   -- 用户2投线代-矩阵运算

-- ============================================
-- 社区模块扩展字段
-- ============================================

-- resources 表扩展：上传者ID（用于"我的上传"查询）、所属科目、审核拒绝原因
ALTER TABLE `resources`
    ADD COLUMN `uploader_id` int DEFAULT NULL COMMENT '上传者用户ID，逻辑关联users表' AFTER `author`;
ALTER TABLE `resources`
    ADD COLUMN `subject` varchar(50) DEFAULT NULL COMMENT '所属科目' AFTER `category`;
ALTER TABLE `resources`
    ADD COLUMN `reject_reason` varchar(500) DEFAULT NULL COMMENT '审核拒绝原因' AFTER `status`;
ALTER TABLE `resources` ADD INDEX `idx_uploader_id` (`uploader_id`);

-- 用户积分表（社区模块打卡等奖励积分累计）
CREATE TABLE IF NOT EXISTS `user_points` (
    `user_id` int NOT NULL COMMENT '用户ID，主键，逻辑关联users表',
    `total_points` int NOT NULL DEFAULT '0' COMMENT '累计积分',
    `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
    PRIMARY KEY (`user_id`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户积分表';

CREATE TABLE `daily_questions` (
                                   `id` int NOT NULL AUTO_INCREMENT COMMENT '题目ID，主键',
                                   `subject` varchar(50) NOT NULL COMMENT '科目（如：高等数学）',
                                   `question` text NOT NULL COMMENT '题目内容',
                                   `options` json NOT NULL COMMENT '选项列表，JSON数组（如：["A选项","B选项","C选项","D选项"]）',
                                   `answer` varchar(10) NOT NULL COMMENT '正确答案索引（如：B）',
                                   `explanation` text COMMENT '答案解析',
                                   `difficulty` tinyint DEFAULT '2' COMMENT '难度等级（1-简单，2-中等，3-困难）',
                                   `date` date NOT NULL COMMENT '题目日期（每日唯一）',
                                   `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                                   PRIMARY KEY (`id`),
                                   UNIQUE KEY `uk_date` (`date`) COMMENT '每日题目唯一',
                                   KEY `idx_subject` (`subject`) COMMENT '科目索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='每日一问题目表';

CREATE TABLE `question_records` (
                                    `id` int NOT NULL AUTO_INCREMENT COMMENT '答题记录ID，主键',
                                    `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                                    `question_id` int NOT NULL COMMENT '题目ID，逻辑关联daily_questions表',
                                    `answered_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '答题时间',
                                    `selected` varchar(10) NOT NULL COMMENT '用户选择的选项（如：B）',
                                    `is_correct` tinyint(1) NOT NULL DEFAULT '0' COMMENT '是否正确（0-错误，1-正确）',
                                    PRIMARY KEY (`id`),
                                    UNIQUE KEY `uk_user_question` (`user_id`, `question_id`) COMMENT '同一用户对同一题目只能答一次',
                                    KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                                    KEY `idx_question_id` (`question_id`) COMMENT '题目ID索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='答题记录表';


CREATE TABLE `notes` (
                         `id` int NOT NULL AUTO_INCREMENT COMMENT '笔记ID，主键',
                         `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                         `title` varchar(50) NOT NULL COMMENT '笔记标题，最多50字',
                         `content` text COMMENT 'Markdown正文内容',
                         `tags` json DEFAULT NULL COMMENT '标签列表，JSON数组（如：["高数","极限"]），最多3个',
                         `is_pinned` tinyint(1) DEFAULT '0' COMMENT '是否置顶（0-否，1-是）',
                         `source_type` varchar(20) DEFAULT 'manual' COMMENT '来源类型（manual-手动创建，resource/note/question-摘录来源类型）',
                         `source_id` int DEFAULT NULL COMMENT '来源ID（资源/帖子/笔记等）',
                         `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                         `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                         PRIMARY KEY (`id`),
                         KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                         KEY `idx_is_pinned` (`is_pinned`) COMMENT '置顶状态索引',
                         KEY `idx_updated_at` (`updated_at`) COMMENT '更新时间索引，用于排序'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='学习笔记表';

CREATE TABLE `semesters` (
                             `id` int NOT NULL AUTO_INCREMENT COMMENT '学期ID，主键',
                             `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                             `name` varchar(20) NOT NULL COMMENT '学期名称（如：大二上）',
                             `year` int NOT NULL COMMENT '学年（如：2025）',
                             `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                             PRIMARY KEY (`id`),
                             UNIQUE KEY `uk_user_name` (`user_id`, `name`, `year`) COMMENT '同一用户学期唯一',
                             KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='学期表';


CREATE TABLE `course_grades` (
                                 `id` int NOT NULL AUTO_INCREMENT COMMENT '课程成绩ID，主键',
                                 `semester_id` int NOT NULL COMMENT '所属学期ID，逻辑关联semesters表',
                                 `name` varchar(50) NOT NULL COMMENT '课程名称',
                                 `credit` decimal(3,1) NOT NULL COMMENT '学分（0.5-10，步长0.5）',
                                 `score` decimal(5,1) DEFAULT NULL COMMENT '百分制成绩（0-100），可为空',
                                 `grade` varchar(5) DEFAULT NULL COMMENT '等级制成绩（A+/A/A-/B+/B/B-/C+/C/C-/D/F），可为空',
                                 `type` varchar(10) NOT NULL DEFAULT '必修' COMMENT '课程类型（必修/选修/公共）',
                                 `gpa` decimal(3,2) DEFAULT NULL COMMENT '绩点（根据算法计算），可为空',
                                 `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                                 `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                                 PRIMARY KEY (`id`),
                                 KEY `idx_semester_id` (`semester_id`) COMMENT '学期ID索引',
                                 KEY `idx_type` (`type`) COMMENT '课程类型索引',
                                 CONSTRAINT `chk_score_or_grade` CHECK ((`score` IS NOT NULL OR `grade` IS NOT NULL))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='课程成绩表';


CREATE TABLE `documents` (
                             `id` int NOT NULL AUTO_INCREMENT COMMENT '文档ID，主键',
                             `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                             `name` varchar(200) NOT NULL COMMENT '文件名',
                             `file_url` varchar(500) NOT NULL COMMENT '云端文件路径',
                             `file_size` bigint NOT NULL DEFAULT '0' COMMENT '文件大小（字节）',
                             `file_type` varchar(20) NOT NULL COMMENT '文件类型（PDF/Word/PPT/Image/Text）',
                             `category` varchar(20) DEFAULT NULL COMMENT '分类（真题/笔记/收藏/导出）',
                             `is_offline` tinyint(1) DEFAULT '0' COMMENT '是否已下载到本地（0-否，1-是）',
                             `last_opened_at` datetime DEFAULT NULL COMMENT '最后打开时间',
                             `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                             `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                             PRIMARY KEY (`id`),
                             KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                             KEY `idx_category` (`category`) COMMENT '分类索引',
                             KEY `idx_file_type` (`file_type`) COMMENT '文件类型索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='个人文档表';


CREATE TABLE `bookmarks` (
                             `id` int NOT NULL AUTO_INCREMENT COMMENT '书签ID，主键',
                             `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
                             `quote` varchar(200) NOT NULL COMMENT '引用原文，最多200字',
                             `source_title` varchar(200) NOT NULL COMMENT '来源标题',
                             `source_url` varchar(500) DEFAULT NULL COMMENT '来源链接',
                             `source_type` varchar(20) NOT NULL COMMENT '来源类型（resource/note/question）',
                             `source_id` int DEFAULT NULL COMMENT '来源ID（对应资源/笔记/问题ID）',
                             `anchor` varchar(100) DEFAULT NULL COMMENT '滚动锚点（如段落ID）',
                             `note` varchar(100) DEFAULT NULL COMMENT '用户备注，最多100字',
                             `color` varchar(10) DEFAULT 'yellow' COMMENT '颜色标签（red/yellow/green/blue/purple）',
                             `created_at` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
                             `updated_at` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
                             PRIMARY KEY (`id`),
                             KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引',
                             KEY `idx_color` (`color`) COMMENT '颜色标签索引',
                             KEY `idx_source_type` (`source_type`) COMMENT '来源类型索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='精选书签表';



ALTER TABLE bookmarks ADD COLUMN updated_at datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP AFTER created_at;

UPDATE bookmarks SET updated_at = created_at WHERE updated_at IS NULL;
ALTER TABLE bookmarks
    MODIFY created_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY updated_at datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;


-- 用户通知设置表（我的页-设置模块）
CREATE TABLE `user_settings` (
    `id` int NOT NULL AUTO_INCREMENT COMMENT '设置ID，主键',
    `user_id` int NOT NULL COMMENT '用户ID，逻辑关联users表',
    `exam_reminder` tinyint(1) NOT NULL DEFAULT '1' COMMENT '考试提醒开关（0-关闭，1-开启）',
    `checkin_reminder` tinyint(1) NOT NULL DEFAULT '1' COMMENT '打卡提醒开关（0-关闭，1-开启）',
    `created_at` datetime DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    `updated_at` datetime DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '最后更新时间',
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_user_id` (`user_id`) COMMENT '用户唯一',
    KEY `idx_user_id` (`user_id`) COMMENT '用户ID索引'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci COMMENT='用户通知设置表';
