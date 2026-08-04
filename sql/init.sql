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