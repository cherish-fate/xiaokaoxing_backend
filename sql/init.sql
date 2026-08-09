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