-- mockbank seed. Idempotent: only runs when `mockbank.users` is empty.
-- Same fixture shape as backendrust/seed.sql so the two demos align.

SET search_path TO mockbank, public;

-- Passwords are all 'demo1234' / 'admin1234' hashed with argon2id via the
-- auth package at first boot. We seed users with a placeholder hash here and
-- the auth.SeedDevUsers() call from main rewrites them with real hashes.
-- That keeps this file pure SQL and avoids depending on a specific Argon2
-- parameter set in static text.

INSERT INTO users (id, username, password_hash, display_name, role,
                   safety_lock_state, safety_lock_reason, failed_mfa_count,
                   risk_tolerance, created_at) VALUES
  ('u_admin',   'admin',        '__argon2_placeholder__', 'Demo Admin',  'admin',    'unlocked', NULL,                0, 'medium', NOW() - INTERVAL '90 days'),
  ('u_demo',    'demo',         '__argon2_placeholder__', 'Alex Demo',   'customer', 'unlocked', NULL,                0, 'medium', NOW() - INTERVAL '60 days'),
  ('u_intl',    'intl_student', '__argon2_placeholder__', 'Mei Intl',    'customer', 'unlocked', NULL,                0, 'low',    NOW() - INTERVAL '45 days'),
  ('u_locked',  'locked_vip',   '__argon2_placeholder__', 'Lin Locked',  'customer', 'locked',   'demo: pre-locked',  3, 'medium', NOW() - INTERVAL '20 days'),
  ('u_auditor', 'auditor',      '__argon2_placeholder__', 'Ops Auditor', 'admin',    'unlocked', NULL,                0, 'low',    NOW() - INTERVAL '10 days');

INSERT INTO accounts (id, user_id, display_name, masked_number, currency,
                      available_balance_fen, reserved_balance_fen, status, created_at) VALUES
  ('acc_demo_chk',    'u_demo',    'Primary checking',  '****1234', 'CNY', 480_000,   0,       'active', NOW() - INTERVAL '60 days'),
  ('acc_demo_sav',    'u_demo',    'Savings',           '****9876', 'CNY', 2_500_000, 50_000,  'active', NOW() - INTERVAL '60 days'),
  ('acc_intl_chk',    'u_intl',    'Student checking',  '****5678', 'CNY', 180_000,   0,       'active', NOW() - INTERVAL '45 days'),
  ('acc_intl_sav',    'u_intl',    'Allowance savings', '****4321', 'CNY', 600_000,   0,       'active', NOW() - INTERVAL '45 days'),
  ('acc_locked_chk',  'u_locked',  'Primary checking',  '****2468', 'CNY', 1_200_000, 0,       'active', NOW() - INTERVAL '20 days'),
  ('acc_auditor_chk', 'u_auditor', 'Ops wallet',        '****0001', 'CNY', 50_000,    0,       'active', NOW() - INTERVAL '10 days');

INSERT INTO payees (id, owner_user_id, display_name, phone_tail, mock_destination, verified, first_used_at, created_at) VALUES
  ('pay_demo_liwei',  'u_demo',  'Li Wei',   '1234', 'mock://Li_Wei',  TRUE, NOW() - INTERVAL '40 days', NOW() - INTERVAL '60 days'),
  ('pay_demo_wangfei','u_demo',  'Wang Fei', '5678', 'mock://Wang_Fei',TRUE, NOW() - INTERVAL '20 days', NOW() - INTERVAL '60 days'),
  ('pay_demo_anna',   'u_demo',  'Anna',     '4321', 'mock://Anna',    TRUE, NULL,                       NOW() - INTERVAL '60 days'),
  ('pay_intl_liwei',  'u_intl',  'Li Wei',   '1234', 'mock://Li_Wei',  TRUE, NOW() - INTERVAL '15 days', NOW() - INTERVAL '45 days'),
  ('pay_intl_anna',   'u_intl',  'Anna',     '4321', 'mock://Anna',    TRUE, NULL,                       NOW() - INTERVAL '45 days'),
  ('pay_locked_x',    'u_locked','Stranger', '9999', 'mock://Stranger',FALSE,NULL,                       NOW() - INTERVAL '20 days');

-- Lightweight transaction history so reports have something to chew on.
-- Explicit inserts (not a generator) so the file runs on any Postgres
-- without a per-tenant random seed dance.
INSERT INTO transactions (id, account_id, amount_fen, direction, category, merchant, memo, occurred_at, status) VALUES
  ('tx_demo_d1',  'acc_demo_chk',  -4500, 'debit', 'food',        'Hotpot Hound',     '', NOW() - INTERVAL '1 day',  'posted'),
  ('tx_demo_d2',  'acc_demo_chk',  -3200, 'debit', 'food',        'Starbucks',         '', NOW() - INTERVAL '1 day',  'posted'),
  ('tx_demo_d3',  'acc_demo_chk',  -8800, 'debit', 'food',        'Family Mart',       '', NOW() - INTERVAL '2 days', 'posted'),
  ('tx_demo_d4',  'acc_demo_chk',  -2200, 'debit', 'transport',   'Metro Card Top-up', '', NOW() - INTERVAL '2 days', 'posted'),
  ('tx_demo_d5',  'acc_demo_chk',  -4500, 'debit', 'subscription','NetStream Premium', '', NOW() - INTERVAL '3 days', 'posted'),
  ('tx_demo_d6',  'acc_demo_chk',  -1500, 'debit', 'subscription','CloudVault 100GB',  '', NOW() - INTERVAL '5 days', 'posted'),
  ('tx_demo_d7',  'acc_demo_chk',  -2900, 'debit', 'subscription','FitCoach Pro',      '', NOW() - INTERVAL '7 days', 'posted'),
  ('tx_demo_d8',  'acc_demo_chk', -12000, 'debit', 'shopping',    'JD Books',          '', NOW() - INTERVAL '4 days', 'posted'),
  ('tx_demo_d9',  'acc_demo_chk',  -6500, 'debit', 'shopping',    'Taobao',            '', NOW() - INTERVAL '6 days', 'posted'),
  ('tx_demo_d10', 'acc_demo_chk',  -3300, 'debit', 'transport',   'Didi',              '', NOW() - INTERVAL '8 days', 'posted'),
  ('tx_demo_d11', 'acc_demo_chk',  -5500, 'debit', 'food',        'Lanzhou Noodles',   '', NOW() - INTERVAL '9 days', 'posted'),
  ('tx_demo_d12', 'acc_demo_chk',  -2500, 'debit', 'food',        'Daily Bread',       '', NOW() - INTERVAL '10 days','posted'),
  ('tx_demo_d13', 'acc_demo_chk',  -4500, 'debit', 'subscription','NetStream Premium', '', NOW() - INTERVAL '12 days','posted'),
  ('tx_demo_d14', 'acc_demo_chk',  -1500, 'debit', 'subscription','CloudVault 100GB',  '', NOW() - INTERVAL '15 days','posted'),
  ('tx_demo_d15', 'acc_demo_chk',  -2900, 'debit', 'subscription','FitCoach Pro',      '', NOW() - INTERVAL '18 days','posted'),
  ('tx_demo_d16', 'acc_demo_chk',  -9800, 'debit', 'shopping',    'Apple Store',       '', NOW() - INTERVAL '20 days','posted'),
  ('tx_demo_d17', 'acc_demo_chk',  -1800, 'debit', 'food',        'Family Mart',       '', NOW() - INTERVAL '22 days','posted'),
  ('tx_demo_d18', 'acc_demo_chk',  -3200, 'debit', 'food',        'Starbucks',         '', NOW() - INTERVAL '25 days','posted'),
  ('tx_intl_d1',  'acc_intl_chk',  -3500, 'debit', 'food',        'Hotpot Hound',      '', NOW() - INTERVAL '1 day',  'posted'),
  ('tx_intl_d2',  'acc_intl_chk',  -1800, 'debit', 'food',        'Daily Bread',       '', NOW() - INTERVAL '3 days', 'posted'),
  ('tx_intl_d3',  'acc_intl_chk',  -4500, 'debit', 'subscription','NetStream Premium', '', NOW() - INTERVAL '4 days', 'posted'),
  ('tx_intl_d4',  'acc_intl_chk',  -2000, 'debit', 'subscription','MusicPass Family',  '', NOW() - INTERVAL '8 days', 'posted'),
  ('tx_intl_d5',  'acc_intl_chk',  -6500, 'debit', 'shopping',    'Taobao',            '', NOW() - INTERVAL '6 days', 'posted'),
  ('tx_intl_d6',  'acc_intl_chk',  -2200, 'debit', 'transport',   'Metro Card Top-up', '', NOW() - INTERVAL '9 days', 'posted'),
  ('tx_intl_d7',  'acc_intl_chk',  -3000, 'debit', 'food',        'Family Mart',       '', NOW() - INTERVAL '14 days','posted'),
  ('tx_intl_d8',  'acc_intl_chk',  -4500, 'debit', 'subscription','NetStream Premium', '', NOW() - INTERVAL '15 days','posted'),
  ('tx_intl_d9',  'acc_intl_chk',  -2000, 'debit', 'subscription','MusicPass Family',  '', NOW() - INTERVAL '27 days','posted');

INSERT INTO subscriptions (id, user_id, merchant, estimated_amount_fen, interval_days,
                           next_expected_at, last_charge_at, evidence, cancellation_status,
                           cancelled_at, created_at) VALUES
  ('sub_demo_ns', 'u_demo',  'NetStream Premium',  4500,  30,
     NOW() + INTERVAL '12 days', NOW() - INTERVAL '18 days',
     '{"rule":"merchant_recurring","hit_count":4}'::jsonb,
     'active', NULL, NOW() - INTERVAL '50 days'),
  ('sub_demo_cv', 'u_demo',  'CloudVault 100GB',   1500,  30,
     NOW() + INTERVAL '8 days',  NOW() - INTERVAL '22 days',
     '{"rule":"merchant_recurring","hit_count":4}'::jsonb,
     'active', NULL, NOW() - INTERVAL '50 days'),
  ('sub_demo_fc', 'u_demo',  'FitCoach Pro',       2900,  30,
     NOW() + INTERVAL '20 days', NOW() - INTERVAL '10 days',
     '{"rule":"merchant_recurring","hit_count":3}'::jsonb,
     'active', NULL, NOW() - INTERVAL '40 days'),
  ('sub_intl_ns', 'u_intl',  'NetStream Premium',  4500,  30,
     NOW() + INTERVAL '15 days', NOW() - INTERVAL '15 days',
     '{"rule":"merchant_recurring","hit_count":3}'::jsonb,
     'active', NULL, NOW() - INTERVAL '35 days'),
  ('sub_intl_mp', 'u_intl',  'MusicPass Family',   2000,  30,
     NOW() + INTERVAL '3 days',  NOW() - INTERVAL '27 days',
     '{"rule":"merchant_recurring","hit_count":3}'::jsonb,
     'cancellation_requested', NOW() - INTERVAL '1 day', NOW() - INTERVAL '30 days');
