// Bilingual string tables. Mirrors `frontend/src/i18n/dictionaries.ts`.
// Flat dot-paths make typos easy to spot and let us reuse the same keys
// the web app exposes.

import 'language.dart';

class Tr {
  final AppLanguage lang;
  final Map<String, String> _dict;

  const Tr._(this.lang, this._dict);

  String t(String key, {Map<String, String>? params}) {
    var s = _dict[key] ?? key;
    if (params != null) {
      params.forEach((k, v) {
        s = s.replaceAll('{$k}', v);
      });
    }
    return s;
  }

  static Tr of(AppLanguage lang) => lang == AppLanguage.zh ? zh : en;

  static final Tr en = Tr._(AppLanguage.en, _en);
  static final Tr zh = Tr._(AppLanguage.zh, _zh);
}

const Map<String, String> _en = {
  // Brand
  'brand.name': 'FinTech Pioneers',
  'brand.tagline': 'Simulation only — no real money.',
  'brand.short': 'FTP',
  'brand.admin': 'FinTech Pioneers Admin',
  'brand.disclaimer':
      'AI may propose. Software decides. No real money is moved.',

  // App shell
  'shell.signed_in_as': 'Signed in as',
  'shell.signout': 'Sign out',
  'shell.locked.title': 'Account locked',
  'shell.locked.body': 'Yellow/Red actions are blocked. Contact support.',

  // Language
  'lang.label': 'Language',
  'lang.english': 'English',
  'lang.chinese': '中文',

  // Login
  'login.title': 'Welcome back',
  'login.subtitle': 'Sign in to your simulation workspace.',
  'login.username': 'Username',
  'login.password': 'Password',
  'login.submit': 'Sign in',
  'login.signingIn': 'Signing in…',
  'login.signInFailed': 'Sign-in failed',
  'login.demo.title': 'Quick demo accounts',
  'login.demo.hint':
      'Tap a row to fill the form. Passwords are shared across the demo seed.',
  'login.hero.body':
      'A simulation of a multilingual banking agent. AI proposes; the policy engine decides; the audit log records everything.',

  // Demo accounts
  'demo.customer.alex.label': 'Customer · Alex Demo',
  'demo.customer.alex.sub':
      'English · medium risk · 2 accounts · 120 days of transactions',
  'demo.customer.mei.label': 'Customer · Mei Intl',
  'demo.customer.mei.sub': 'Mixed EN/中文 · low risk · student accounts',
  'demo.customer.vera.label': 'Customer · Vera VIP',
  'demo.customer.vera.sub': 'High balance · equity + bond holdings',
  'demo.admin.label': 'Admin · Demo Admin',
  'demo.admin.sub': 'Audit · safety · user unlock · overview',
  'demo.locked.label': 'Locked · Lin Locked',
  'demo.locked.sub': '3 denied high-risk attempts — Yellow/Red blocked',

  // Common
  'common.cancel': 'Cancel',
  'common.confirm': 'Confirm',
  'common.save': 'Save',
  'common.delete': 'Delete',
  'common.loading': 'Loading…',
  'common.thinking': 'Thinking…',
  'common.send': 'Send',
  'common.retry': 'Retry',
  'common.yes': 'Yes',
  'common.no': 'No',
  'common.unauthorized': 'Sign in first.',
  'common.refresh': 'Refresh',
  'common.back': 'Back',
  'common.continue': 'Continue',
  'common.optional': 'optional',
  'common.amount': 'Amount',

  // Nav
  'nav.dashboard': 'Dashboard',
  'nav.assistant': 'Assistant',
  'nav.transfers': 'Transfers',
  'nav.cards': 'Cards',
  'nav.wealth': 'Wealth',
  'nav.more': 'More',

  // Tiers
  'tier.green': 'Green',
  'tier.yellow': 'Yellow',
  'tier.red': 'Red',
  'tier.green.desc': 'Auto-approved',
  'tier.yellow.desc': 'Needs your confirmation',
  'tier.red.desc': 'Confirmation + mock MFA',

  // Dashboard
  'dashboard.title': 'Dashboard',
  'dashboard.hello': 'Hello, {name}',
  'dashboard.total_balance': 'Total balance',
  'dashboard.accounts': 'Accounts',
  'dashboard.recent': 'Recent transactions',
  'dashboard.spending': 'Spending this month',
  'dashboard.no_transactions': 'No transactions yet.',
  'dashboard.view_all': 'View all',

  // Accounts
  'accounts.title': 'Accounts',
  'accounts.available': 'Available',
  'accounts.reserved': 'Reserved',
  'accounts.total': 'Total',

  // Chat
  'chat.title': 'Assistant',
  'chat.placeholder': 'Ask in English, 中文, or mixed…',
  'chat.empty': 'Try: "转 200 给 Li Wei，午饭钱"',
  'chat.suggestion.transfer': 'Pay Li Wei ¥200 for lunch',
  'chat.suggestion.bill': 'How much did I spend on dining last month?',
  'chat.suggestion.subscription': 'Show my subscriptions',
  'chat.suggestion.card': 'Issue a virtual card for online shopping',
  'chat.pending_action': 'Action requires your approval',
  'chat.approve': 'Approve',
  'chat.deny': 'Deny',
  'chat.mfa.label': 'Enter the 6-digit mock MFA code',
  'chat.mfa.submit': 'Verify',
  'chat.mfa.hint': 'Any 6-digit code is accepted in the simulation.',
  'chat.executed': 'Action completed.',
  'chat.clarification': 'The agent needs more information to continue.',
  'chat.workflow_created': 'Plan created',

  // Transfers
  'transfers.title': 'Transfers',
  'transfers.new': 'New transfer',
  'transfers.recipient': 'Recipient',
  'transfers.amount': 'Amount (CNY)',
  'transfers.memo': 'Memo (optional)',
  'transfers.preview': 'Preview',
  'transfers.send': 'Send',
  'transfers.history': 'History',
  'transfers.empty': 'No transfers yet.',
  'transfers.preview.tier': 'Risk tier',
  'transfers.preview.remaining': 'Daily remaining',
  'transfers.preview.recipient': 'Resolved recipient',
  'transfers.success': 'Transfer posted.',

  // Cards
  'cards.title': 'Virtual cards',
  'cards.issue': 'Issue a new card',
  'cards.issue.name': 'Card nickname',
  'cards.issue.submit': 'Issue',
  'cards.freeze': 'Freeze',
  'cards.unfreeze': 'Unfreeze',
  'cards.cancel': 'Cancel',
  'cards.report_lost': 'Report lost',
  'cards.status.issued': 'Issued',
  'cards.status.active': 'Active',
  'cards.status.frozen': 'Frozen',
  'cards.status.lost': 'Lost',
  'cards.status.cancelled': 'Cancelled',
  'cards.status.expired': 'Expired',
  'cards.empty': 'No virtual cards yet.',

  // Wealth
  'wealth.title': 'Wealth',
  'wealth.products': 'Catalog',
  'wealth.holdings': 'Holdings',
  'wealth.subscribe': 'Subscribe',
  'wealth.redeem': 'Redeem',
  'wealth.expected_return': 'Expected return',
  'wealth.min': 'Minimum',
  'wealth.risk': 'Risk',
  'wealth.empty.products': 'Catalog is empty.',
  'wealth.empty.holdings': 'You have no holdings yet.',
  'wealth.subscribe.amount': 'Amount (CNY)',
  'wealth.subscribe.suitable': 'Suitability check passed',
  'wealth.subscribe.not_suitable': 'Suitability check failed',

  // More
  'more.title': 'More',
  'more.subscriptions': 'Subscriptions',
  'more.budgets': 'Budgets',
  'more.forecast': 'Forecast',
  'more.audit': 'Audit ledger',
  'more.about': 'About',
  'more.language': 'Language',
  'more.signout': 'Sign out',
  'more.sim_banner': 'Simulation only — no real bank accounts, no real payments.',

  // Subscriptions
  'subs.title': 'Subscriptions',
  'subs.next': 'Next charge',
  'subs.estimated': 'Estimated',
  'subs.cancel': 'Cancel subscription',
  'subs.cancel.confirm':
      'Send a simulated cancellation to the merchant? No real money is moved.',
  'subs.empty': 'No recurring charges detected.',

  // Budgets
  'budgets.title': 'Budgets',
  'budgets.remaining': 'Remaining',
  'budgets.spent': 'Spent',
  'budgets.limit': 'Limit',
  'budgets.empty': 'No budgets set.',

  // Forecast
  'forecast.title': 'Forecast',
  'forecast.run': 'Simulate',
  'forecast.amount': 'Amount (CNY)',
  'forecast.verdict.yes': 'Yes, you can afford it',
  'forecast.verdict.tight': 'Tight — borderline',
  'forecast.verdict.no': 'No — not recommended',

  // Audit
  'audit.title': 'Audit ledger',
  'audit.empty': 'No audit events.',
  'audit.action': 'Action',
  'audit.tool': 'Tool',
  'audit.tier': 'Tier',
  'audit.time': 'Time',

  // Errors
  'error.network': 'Network error. Check your connection.',
  'error.unknown': 'Something went wrong.',
  'error.session_expired': 'Your session has expired. Sign in again.',

  // About
  'about.version': 'Version',
  'about.build': 'Build',
  'about.api': 'API',
  'about.disclaimer':
      'This is a simulation. No real bank accounts, no real payments, no real MFA. AI may propose; software decides.',
};

const Map<String, String> _zh = {
  // Brand
  'brand.name': '智银助手',
  'brand.tagline': '仅模拟，不涉及真实资金。',
  'brand.short': '智银',
  'brand.admin': '智银助手 · 管理端',
  'brand.disclaimer': 'AI 可提议，规则做决定，不动真金白银。',

  // App shell
  'shell.signed_in_as': '已登录',
  'shell.signout': '退出登录',
  'shell.locked.title': '账户已锁定',
  'shell.locked.body': 'Yellow/Red 操作已被拦截，请联系客服。',

  // Language
  'lang.label': '语言',
  'lang.english': 'English',
  'lang.chinese': '中文',

  // Login
  'login.title': '欢迎回来',
  'login.subtitle': '登录你的模拟工作台。',
  'login.username': '用户名',
  'login.password': '密码',
  'login.submit': '登录',
  'login.signingIn': '正在登录…',
  'login.signInFailed': '登录失败',
  'login.demo.title': '演示账号',
  'login.demo.hint': '点击任一行可快速填入。演示种子共享同一密码。',
  'login.hero.body':
      '多语种银行智能体模拟：AI 提议，策略引擎决定，审计日志全程留痕。',

  // Demo accounts
  'demo.customer.alex.label': '客户 · 演示 Alex',
  'demo.customer.alex.sub': '英语 · 中等风险 · 2 个账户 · 120 天流水',
  'demo.customer.mei.label': '客户 · 留学生 Mei',
  'demo.customer.mei.sub': '中英混合 · 低风险 · 学生账户',
  'demo.customer.vera.label': '客户 · 高净值 Vera',
  'demo.customer.vera.sub': '高余额 · 股 + 债持仓',
  'demo.admin.label': '管理员 · 演示 Admin',
  'demo.admin.sub': '审计 · 风控 · 用户解锁 · 总览',
  'demo.locked.label': '已锁定 · Lin Locked',
  'demo.locked.sub': '3 次高风险拒绝 — Yellow/Red 已被拦截',

  // Common
  'common.cancel': '取消',
  'common.confirm': '确认',
  'common.save': '保存',
  'common.delete': '删除',
  'common.loading': '加载中…',
  'common.thinking': '思考中…',
  'common.send': '发送',
  'common.retry': '重试',
  'common.yes': '是',
  'common.no': '否',
  'common.unauthorized': '请先登录。',
  'common.refresh': '刷新',
  'common.back': '返回',
  'common.continue': '继续',
  'common.optional': '可选',
  'common.amount': '金额',

  // Nav
  'nav.dashboard': '总览',
  'nav.assistant': '助手',
  'nav.transfers': '转账',
  'nav.cards': '卡片',
  'nav.wealth': '理财',
  'nav.more': '更多',

  // Tiers
  'tier.green': '绿',
  'tier.yellow': '黄',
  'tier.red': '红',
  'tier.green.desc': '自动放行',
  'tier.yellow.desc': '需你确认',
  'tier.red.desc': '确认 + 模拟 MFA',

  // Dashboard
  'dashboard.title': '总览',
  'dashboard.hello': '你好，{name}',
  'dashboard.total_balance': '总余额',
  'dashboard.accounts': '账户',
  'dashboard.recent': '近期交易',
  'dashboard.spending': '本月支出',
  'dashboard.no_transactions': '暂无交易。',
  'dashboard.view_all': '查看全部',

  // Accounts
  'accounts.title': '账户',
  'accounts.available': '可用',
  'accounts.reserved': '冻结',
  'accounts.total': '总额',

  // Chat
  'chat.title': '助手',
  'chat.placeholder': '用中文、英文或混合输入…',
  'chat.empty': '试试："转 200 给李伟，午饭钱"',
  'chat.suggestion.transfer': '给李伟转 200 块午饭钱',
  'chat.suggestion.bill': '上个月餐饮花了多少？',
  'chat.suggestion.subscription': '看看我的订阅',
  'chat.suggestion.card': '给我开一张虚拟卡用于网购',
  'chat.pending_action': '此操作需要你确认',
  'chat.approve': '批准',
  'chat.deny': '拒绝',
  'chat.mfa.label': '输入 6 位模拟 MFA 码',
  'chat.mfa.submit': '验证',
  'chat.mfa.hint': '模拟环境：任意 6 位数字均可通过。',
  'chat.executed': '已完成。',
  'chat.clarification': '智能体需要更多信息。',
  'chat.workflow_created': '已生成计划',

  // Transfers
  'transfers.title': '转账',
  'transfers.new': '新建转账',
  'transfers.recipient': '收款人',
  'transfers.amount': '金额 (CNY)',
  'transfers.memo': '备注（可选）',
  'transfers.preview': '预览',
  'transfers.send': '发送',
  'transfers.history': '历史',
  'transfers.empty': '暂无转账记录。',
  'transfers.preview.tier': '风险等级',
  'transfers.preview.remaining': '当日剩余额度',
  'transfers.preview.recipient': '收款方',
  'transfers.success': '转账已发出。',

  // Cards
  'cards.title': '虚拟卡',
  'cards.issue': '开新卡',
  'cards.issue.name': '卡片昵称',
  'cards.issue.submit': '开卡',
  'cards.freeze': '冻结',
  'cards.unfreeze': '解冻',
  'cards.cancel': '注销',
  'cards.report_lost': '挂失',
  'cards.status.issued': '已开',
  'cards.status.active': '正常',
  'cards.status.frozen': '已冻结',
  'cards.status.lost': '已挂失',
  'cards.status.cancelled': '已注销',
  'cards.status.expired': '已过期',
  'cards.empty': '暂无虚拟卡。',

  // Wealth
  'wealth.title': '理财',
  'wealth.products': '产品目录',
  'wealth.holdings': '持仓',
  'wealth.subscribe': '认购',
  'wealth.redeem': '赎回',
  'wealth.expected_return': '预期年化',
  'wealth.min': '起购',
  'wealth.risk': '风险',
  'wealth.empty.products': '暂无产品。',
  'wealth.empty.holdings': '暂无持仓。',
  'wealth.subscribe.amount': '金额 (CNY)',
  'wealth.subscribe.suitable': '通过适当性校验',
  'wealth.subscribe.not_suitable': '未通过适当性校验',

  // More
  'more.title': '更多',
  'more.subscriptions': '订阅',
  'more.budgets': '预算',
  'more.forecast': '预测',
  'more.audit': '审计',
  'more.about': '关于',
  'more.language': '语言',
  'more.signout': '退出',
  'more.sim_banner': '仅模拟，不涉及真实账户与资金。',

  // Subscriptions
  'subs.title': '订阅',
  'subs.next': '下次扣款',
  'subs.estimated': '金额',
  'subs.cancel': '取消订阅',
  'subs.cancel.confirm': '向商户发送模拟取消请求，不动真金白银。',
  'subs.empty': '未检测到周期性扣款。',

  // Budgets
  'budgets.title': '预算',
  'budgets.remaining': '剩余',
  'budgets.spent': '已花',
  'budgets.limit': '上限',
  'budgets.empty': '尚未设置预算。',

  // Forecast
  'forecast.title': '预测',
  'forecast.run': '模拟',
  'forecast.amount': '金额 (CNY)',
  'forecast.verdict.yes': '可以负担',
  'forecast.verdict.tight': '偏紧，勉强可负担',
  'forecast.verdict.no': '不建议',

  // Audit
  'audit.title': '审计',
  'audit.empty': '暂无审计事件。',
  'audit.action': '操作',
  'audit.tool': '工具',
  'audit.tier': '等级',
  'audit.time': '时间',

  // Errors
  'error.network': '网络异常，请检查连接。',
  'error.unknown': '出错了。',
  'error.session_expired': '会话已过期，请重新登录。',

  // About
  'about.version': '版本',
  'about.build': '构建',
  'about.api': 'API',
  'about.disclaimer': '仅模拟，无真实账户、真实支付、真实 MFA。AI 可提议，规则做决定。',
};
