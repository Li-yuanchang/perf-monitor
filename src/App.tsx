import { useState, useEffect, useRef } from 'react';
import './style.css';
import { invoke } from '@tauri-apps/api/tauri';
import * as echarts from 'echarts';

interface Metric {
  id: number;
  timestamp: string;
  cpu: number;
  memory: number;
  swap_used: number;
  disk_usage: number;
  load_avg: number;
}

interface ProcessInfo {
  pid: number;
  name: string;
  cpu: number;
  mem: number;
  rss: number;
}

interface TimeMark {
  id: number;
  name: string;
  timestamp: string;
  note: string;
}

interface Report {
  before_window: TimeWindowStats;
  after_window: TimeWindowStats;
  comparison: ComparisonStats;
}

interface TimeWindowStats {
  start: string;
  end: string;
  avg_cpu: number;
  peak_cpu: number;
  avg_memory: number;
  peak_memory: number;
  avg_swap: number;
  peak_swap: number;
  avg_load: number;
  sample_count: number;
}

interface ComparisonStats {
  cpu_better: boolean;
  cpu_better_percent: number;
  mem_better: boolean;
  mem_better_percent: number;
  swap_better: boolean;
  swap_better_percent: number;
  lag_score_before: number;
  lag_score_after: number;
  conclusion: string;
}

interface ProcessRanking {
  name: string;
  avg_cpu: number;
  peak_cpu: number;
  avg_mem: number;
  peak_mem: number;
  avg_rss_mb: number;
  peak_rss_mb: number;
  sample_count: number;
}

type TabType = 'realtime' | 'history' | 'processes' | 'marks' | 'report' | 'settings';

function App() {
  const [isExpanded, setIsExpanded] = useState(false);
  const [metric, setMetric] = useState<Metric | null>(null);

  // Tabs
  const [activeTab, setActiveTab] = useState<TabType>('realtime');

  // History data
  const [historyData, setHistoryData] = useState<Metric[]>([]);
  const [historyRange, setHistoryRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h');
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  // Processes
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [processFilter, setProcessFilter] = useState('');
  const [sortKey, setSortKey] = useState<'pid' | 'name' | 'cpu' | 'mem' | 'rss'>('cpu');
  const [sortAsc, setSortAsc] = useState(false);

  // Time marks
  const [marks, setMarks] = useState<TimeMark[]>([]);
  const [newMarkName, setNewMarkName] = useState('');

  // Report
  const [report, setReport] = useState<Report | null>(null);
  const [beforeWindow, setBeforeWindow] = useState({ minutes: 10 });
  const [afterWindow, setAfterWindow] = useState({ minutes: 10 });

  // Process rankings
  const [rankings, setRankings] = useState<ProcessRanking[]>([]);
  const [rankingRange, setRankingRange] = useState<'1h' | '6h' | '24h'>('1h');

  // Settings
  const [settings, setSettings] = useState({
    collectionInterval: 3,
    retentionDays: 30,
    processInterval: 30,
  });
  const [autostart, setAutostart] = useState(false);
  const [theme, setTheme] = useState<'auto' | 'light' | 'dark'>('auto');

  // Fetch realtime data (with mock fallback)
  useEffect(() => {
    const mockMetric: Metric = {
      id: 1,
      timestamp: new Date().toISOString(),
      cpu: 35.5,
      memory: 62.3,
      swap_used: 512,
      disk_usage: 78.2,
      load_avg: 1.23,
    };

    const fetchData = () => {
      invoke<Metric | null>('get_realtime')
        .then((data) => {
          if (data) setMetric(data);
        })
        .catch(() => {
          // Mock mode when Tauri is not available
          setMetric({
            ...mockMetric,
            timestamp: new Date().toISOString(),
            cpu: 20 + Math.random() * 40,
            memory: 50 + Math.random() * 20,
          });
        });
    };

    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, []);

  // Fetch processes
  useEffect(() => {
    if (isExpanded && activeTab === 'processes') {
      const mockProcesses: ProcessInfo[] = [
        { pid: 1, name: 'kernel_task', cpu: 8.5, mem: 12.3, rss: 2147483648 },
        { pid: 4523, name: 'Chrome', cpu: 25.2, mem: 18.5, rss: 3221225472 },
        { pid: 2341, name: 'VS Code', cpu: 15.8, mem: 8.2, rss: 1436549120 },
        { pid: 892, name: 'Docker', cpu: 5.3, mem: 6.1, rss: 1073741824 },
        { pid: 3412, name: 'Slack', cpu: 3.2, mem: 4.5, rss: 805306368 },
        { pid: 1234, name: 'Spotify', cpu: 2.1, mem: 3.2, rss: 536870912 },
        { pid: 5678, name: 'Terminal', cpu: 1.5, mem: 1.8, rss: 268435456 },
        { pid: 9012, name: 'Finder', cpu: 0.8, mem: 2.1, rss: 134217728 },
      ];

      const fetchProcesses = () => {
        invoke<ProcessInfo[]>('get_processes')
          .then((data) => {
            setProcesses(data);
          })
          .catch(() => {
            setProcesses(mockProcesses);
          });
      };

      fetchProcesses();
      const interval = setInterval(fetchProcesses, 15000);
      return () => clearInterval(interval);
    }
  }, [isExpanded, activeTab]);

  // Fetch history data
  useEffect(() => {
    if (isExpanded && activeTab === 'history') {
      const end = new Date();
      const start = new Date();
      switch (historyRange) {
        case '1h': start.setHours(end.getHours() - 1); break;
        case '6h': start.setHours(end.getHours() - 6); break;
        case '24h': start.setDate(end.getDate() - 1); break;
        case '7d': start.setDate(end.getDate() - 7); break;
      }

      invoke<Metric[]>('get_history', {
        start: start.toISOString(),
        end: end.toISOString(),
      })
        .then((data) => setHistoryData(data))
        .catch(() => {
          // Generate mock history data
          const mockData: Metric[] = [];
          const points = historyRange === '1h' ? 60 : historyRange === '6h' ? 72 : historyRange === '24h' ? 96 : 84;
          for (let i = points; i >= 0; i--) {
            const time = new Date(end.getTime() - i * (historyRange === '1h' ? 60000 : historyRange === '6h' ? 300000 : historyRange === '24h' ? 900000 : 3600000));
            mockData.push({
              id: i,
              timestamp: time.toISOString(),
              cpu: 20 + Math.random() * 30 + Math.sin(i / 10) * 10,
              memory: 55 + Math.random() * 15,
              swap_used: 400 + Math.random() * 200,
              disk_usage: 75 + Math.random() * 5,
              load_avg: 1 + Math.random(),
            });
          }
          setHistoryData(mockData);
        });
    }
  }, [isExpanded, activeTab, historyRange]);

  // Render chart
  useEffect(() => {
    if (isExpanded && activeTab === 'history' && chartRef.current && historyData.length > 0) {
      // 每次都重新创建实例（收起时 DOM 会被销毁）
      if (chartInstance.current) {
        chartInstance.current.dispose();
        chartInstance.current = null;
      }
      chartInstance.current = echarts.init(chartRef.current);

      const times = historyData.map((m) => new Date(m.timestamp).toLocaleTimeString());
      const cpus = historyData.map((m) => m.cpu);
      const memories = historyData.map((m) => m.memory);
      const swaps = historyData.map((m) => m.swap_used);
      const disks = historyData.map((m) => m.disk_usage);

      chartInstance.current.setOption({
        tooltip: {
          trigger: 'axis',
          formatter: (params: any) => {
            let s = `${params[0].axisValue}<br/>`;
            for (const p of params) {
              const unit = p.seriesName === 'Swap(MB)' ? ' MB' : '%';
              s += `${p.marker} ${p.seriesName}&nbsp;&nbsp;<b>${Math.round(p.value)}${unit}</b><br/>`;
            }
            return s;
          },
        },
        legend: { data: ['CPU%', '内存%', '磁盘%', 'Swap(MB)'], top: 0 },
        grid: { left: 50, right: 60, top: 40, bottom: 50 },
        xAxis: {
          type: 'category',
          data: times,
          axisLabel: {
            rotate: 45,
            fontSize: 10,
            interval: Math.max(0, Math.floor(times.length / 12) - 1),
          },
        },
        yAxis: [
          { type: 'value', name: '百分比(%)', min: 0, max: 100 },
          { type: 'value', name: 'Swap(MB)', position: 'right', splitLine: { show: false } },
        ],
        series: [
          { name: 'CPU%', type: 'line', data: cpus, smooth: true, symbol: 'none', yAxisIndex: 0 },
          { name: '内存%', type: 'line', data: memories, smooth: true, symbol: 'none', yAxisIndex: 0 },
          { name: '磁盘%', type: 'line', data: disks, smooth: true, symbol: 'none', yAxisIndex: 0 },
          { name: 'Swap(MB)', type: 'line', data: swaps, smooth: true, symbol: 'none', yAxisIndex: 1 },
        ],
      });
    }

    return () => {
      if (chartInstance.current) {
        chartInstance.current.dispose();
        chartInstance.current = null;
      }
    };
  }, [isExpanded, activeTab, historyData]);

  // Fetch marks
  useEffect(() => {
    if (isExpanded && activeTab === 'marks') {
      fetchMarks();
    }
  }, [isExpanded, activeTab]);

  // Fetch process rankings
  useEffect(() => {
    if (isExpanded && activeTab === 'report') {
      const hours = rankingRange === '1h' ? 1 : rankingRange === '6h' ? 6 : 24;
      const end = new Date();
      const start = new Date(end.getTime() - hours * 3600000);
      invoke<ProcessRanking[]>('get_process_ranking', {
        start: start.toISOString(),
        end: end.toISOString(),
      })
        .then((data) => setRankings(data))
        .catch(() => setRankings([]));
    }
  }, [isExpanded, activeTab, rankingRange]);

  // Theme: apply data-theme attribute to html element
  useEffect(() => {
    if (theme === 'auto') {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', theme);
    }
    localStorage.setItem('perf-theme', theme);
  }, [theme]);

  // Load saved theme & autostart status on mount
  useEffect(() => {
    const saved = localStorage.getItem('perf-theme') as 'auto' | 'light' | 'dark' | null;
    if (saved) setTheme(saved);
    invoke<boolean>('get_autostart').then(setAutostart).catch(() => {});
  }, []);

  const fetchMarks = () => {
    invoke<TimeMark[]>('get_marks')
      .then((data) => setMarks(data))
      .catch(() => {
        setMarks([
          { id: 1, name: '系统升级前', timestamp: new Date(Date.now() - 86400000).toISOString(), note: '' },
          { id: 2, name: '应用发布前', timestamp: new Date(Date.now() - 43200000).toISOString(), note: '' },
        ]);
      });
  };

  // 区分点击和拖动：快速点击(<200ms)=展开，长按=拖动
  const mouseDownTime = useRef<number>(0);

  const handleBallMouseDown = () => {
    mouseDownTime.current = Date.now();
  };

  const handleBallMouseUp = async () => {
    const duration = Date.now() - mouseDownTime.current;
    mouseDownTime.current = 0;
    if (duration > 200) return;
    setIsExpanded(true);
    try { await invoke('expand_window'); } catch {}
  };

  const doCollapse = async () => {
    setIsExpanded(false);
    try { await invoke('collapse_window'); } catch {}
  };

  const getCpuColor = (cpu: number) => {
    if (cpu > 80) return '#ff453a';
    if (cpu > 50) return '#ff9f0a';
    return '#30d158';
  };

  const addMark = () => {
    if (!newMarkName.trim()) return;
    invoke('set_mark', { name: newMarkName.trim() }).then(() => {
      setNewMarkName('');
      fetchMarks();
    });
  };

  const generateReport = () => {
    // 以当前时间为分界点，前N分钟 vs 后N分钟（后窗口=最近N分钟）
    const now = new Date();
    const beforeEnd = new Date(now.getTime() - afterWindow.minutes * 60000);
    const beforeStart = new Date(beforeEnd.getTime() - beforeWindow.minutes * 60000);
    const afterStart = beforeEnd;
    const afterEnd = now;

    invoke<Report>('generate_report', {
      beforeStart: beforeStart.toISOString(),
      beforeEnd: beforeEnd.toISOString(),
      afterStart: afterStart.toISOString(),
      afterEnd: afterEnd.toISOString(),
    })
      .then((data) => setReport(data))
      .catch((err) => {
        alert(`生成报告失败: ${err}`);
      });
  };

  const exportCSV = () => {
    const end = new Date();
    const start = new Date();
    start.setDate(end.getDate() - 1);

    invoke<string>('export_csv', {
      start: start.toISOString(),
      end: end.toISOString(),
    }).then((filepath) => {
      alert(`已导出到: ${filepath}`);
    });
  };

  // Floating ball mode - water wave ball
  if (!isExpanded) {
    const cpuVal = metric?.cpu || 0;
    const waterLevel = 64 - (cpuVal / 100) * 64;

    // 波浪颜色随 CPU 变化：绿→黄→红
    const waveFront = cpuVal > 80 ? 'rgba(255,69,58,0.6)' : cpuVal > 50 ? 'rgba(255,159,10,0.55)' : 'rgba(48,209,88,0.55)';
    const waveBack = cpuVal > 80 ? 'rgba(255,69,58,0.35)' : cpuVal > 50 ? 'rgba(255,159,10,0.3)' : 'rgba(48,209,88,0.3)';
    const borderColor = cpuVal > 80 ? 'rgba(255,69,58,0.5)' : cpuVal > 50 ? 'rgba(255,159,10,0.4)' : 'rgba(48,209,88,0.35)';

    return (
      <div
        className="floating-ball"
        onMouseDown={handleBallMouseDown}
        onMouseUp={handleBallMouseUp}
        onContextMenu={(e) => e.preventDefault()}
      >
        <svg className="water-svg" viewBox="0 0 64 64">
          <defs>
            <clipPath id="ball-clip">
              <circle cx="32" cy="32" r="30" />
            </clipPath>
          </defs>
          <g clipPath="url(#ball-clip)">
            <circle cx="32" cy="32" r="30" className="water-bg" />
            <g style={{ transform: `translateY(${waterLevel - 32}px)` }} className="water-level">
              <path className="wave wave-back" style={{ fill: waveBack }}
                d="M-48,32 C-40,28 -32,36 -24,32 C-16,28 -8,36 0,32 C8,28 16,36 24,32 C32,28 40,36 48,32 C56,28 64,36 72,32 C80,28 88,36 96,32 C104,28 112,36 120,32 L120,96 L-48,96 Z"
              />
              <path className="wave wave-front" style={{ fill: waveFront }}
                d="M-60,32 C-50,36 -40,28 -30,32 C-20,36 -10,28 0,32 C10,36 20,28 30,32 C40,36 50,28 60,32 C70,36 80,28 90,32 C100,36 110,28 120,32 L120,96 L-60,96 Z"
              />
            </g>
          </g>
          <circle cx="32" cy="32" r="30" className="water-border" style={{ stroke: borderColor }} />
        </svg>

        <div className="ball-content">
          <div className="ball-number" style={{ color: getCpuColor(cpuVal) }}>
            {metric ? Math.round(cpuVal) : '--'}
          </div>
          <div className="ball-unit">%</div>
        </div>
      </div>
    );
  }

  // Expanded mode
  return (
    <div className="app">
      <header className="app-header" data-tauri-drag-region>
        <div className="header-left" data-tauri-drag-region>
          <button className="collapse-btn" onClick={doCollapse} title="收起">
            ←
          </button>
          <h1 data-tauri-drag-region>性能监控</h1>
        </div>
        <nav className="tab-nav">
          {[
            { key: 'realtime', label: '实时' },
            { key: 'history', label: '历史' },
            { key: 'processes', label: '进程' },
            { key: 'marks', label: '标记' },
            { key: 'report', label: '报告' },
            { key: 'settings', label: '设置' },
          ].map((tab) => (
            <button
              key={tab.key}
              className={`tab-btn ${activeTab === tab.key ? 'active' : ''}`}
              onClick={() => setActiveTab(tab.key as TabType)}
            >
              {tab.label}
            </button>
          ))}
        </nav>
      </header>

      <main className="app-main">
        {/* Realtime Tab */}
        {activeTab === 'realtime' && (
          <div className="realtime-panel">
            <div className="metrics-grid">
              <div className={`metric-card ${(metric?.cpu || 0) > 80 ? 'alert' : ''}`}>
                <h3>CPU 使用率</h3>
                <div className="metric-value" style={{ color: getCpuColor(metric?.cpu || 0) }}>
                  {metric ? metric.cpu.toFixed(1) : '--'}%
                </div>
                <div className="metric-bar">
                  <div className="metric-bar-fill" style={{ width: `${metric?.cpu || 0}%`, background: getCpuColor(metric?.cpu || 0) }} />
                </div>
                {(metric?.cpu || 0) > 80 && <div className="metric-alert">CPU 过高！查看进程 →</div>}
              </div>
              <div className={`metric-card ${(metric?.memory || 0) > 85 ? 'alert' : ''}`}>
                <h3>内存使用率</h3>
                <div className="metric-value" style={{ color: (metric?.memory || 0) > 80 ? '#ff453a' : (metric?.memory || 0) > 60 ? '#ff9f0a' : '#30d158' }}>
                  {metric ? metric.memory.toFixed(1) : '--'}%
                </div>
                <div className="metric-bar">
                  <div className="metric-bar-fill" style={{ width: `${metric?.memory || 0}%`, background: (metric?.memory || 0) > 80 ? '#ff453a' : (metric?.memory || 0) > 60 ? '#ff9f0a' : '#30d158' }} />
                </div>
                {(metric?.memory || 0) > 85 && <div className="metric-alert">内存紧张！</div>}
              </div>
              <div className="metric-card">
                <h3>Swap 使用</h3>
                <div className="metric-value" style={{ color: (metric?.swap_used || 0) > 2000 ? '#ff9f0a' : '#30d158' }}>
                  {metric ? metric.swap_used?.toFixed(0) : '--'} MB
                </div>
                <div className="metric-bar">
                  <div className="metric-bar-fill" style={{ width: `${Math.min(((metric?.swap_used || 0) / 8192) * 100, 100)}%`, background: (metric?.swap_used || 0) > 2000 ? '#ff9f0a' : '#0a84ff' }} />
                </div>
              </div>
              <div className={`metric-card ${(metric?.disk_usage || 0) > 90 ? 'alert' : ''}`}>
                <h3>磁盘使用率</h3>
                <div className="metric-value" style={{ color: (metric?.disk_usage || 0) > 90 ? '#ff453a' : '#30d158' }}>
                  {metric ? metric.disk_usage?.toFixed(1) : '--'}%
                </div>
                <div className="metric-bar">
                  <div className="metric-bar-fill" style={{ width: `${metric?.disk_usage || 0}%`, background: (metric?.disk_usage || 0) > 90 ? '#ff453a' : (metric?.disk_usage || 0) > 75 ? '#ff9f0a' : '#30d158' }} />
                </div>
                {(metric?.disk_usage || 0) > 90 && <div className="metric-alert">磁盘空间不足！</div>}
              </div>
              <div className="metric-card">
                <h3>系统负载</h3>
                <div className="metric-value" style={{ color: (metric?.load_avg || 0) > 4 ? '#ff453a' : (metric?.load_avg || 0) > 2 ? '#ff9f0a' : '#30d158' }}>
                  {metric ? metric.load_avg?.toFixed(2) : '--'}
                </div>
                <div className="metric-bar">
                  <div className="metric-bar-fill" style={{ width: `${Math.min(((metric?.load_avg || 0) / 8) * 100, 100)}%`, background: (metric?.load_avg || 0) > 4 ? '#ff453a' : (metric?.load_avg || 0) > 2 ? '#ff9f0a' : '#0a84ff' }} />
                </div>
              </div>
            </div>
            <div className="realtime-actions">
              <button className="action-btn" onClick={() => setActiveTab('processes')}>查看进程</button>
              <button className="action-btn" onClick={() => setActiveTab('history')}>查看历史</button>
              <button className="action-btn" onClick={exportCSV}>导出数据</button>
            </div>
          </div>
        )}

        {/* History Tab */}
        {activeTab === 'history' && (
          <div className="history-panel">
            <div className="panel-header">
              <div className="range-selector">
                {[
                  { key: '1h', label: '1小时' },
                  { key: '6h', label: '6小时' },
                  { key: '24h', label: '24小时' },
                  { key: '7d', label: '7天' },
                ].map((range) => (
                  <button
                    key={range.key}
                    className={`range-btn ${historyRange === range.key ? 'active' : ''}`}
                    onClick={() => setHistoryRange(range.key as typeof historyRange)}
                  >
                    {range.label}
                  </button>
                ))}
              </div>
              <button className="action-btn" onClick={exportCSV}>导出CSV</button>
            </div>
            <div ref={chartRef} className="chart-container" />
          </div>
        )}

        {/* Processes Tab */}
        {activeTab === 'processes' && (() => {
          const appNames: Record<string, string> = {
            'kernel_task': '系统内核', 'WindowServer': '窗口服务', 'loginwindow': '登录窗口',
            'Finder': '访达', 'Safari': 'Safari 浏览器', 'safariservicesagent': 'Safari 服务',
            'Google Chrome': 'Chrome 浏览器', 'Google Chrome Helper': 'Chrome 辅助',
            'Chrome': 'Chrome 浏览器', 'firefox': 'Firefox 浏览器',
            'Mail': '邮件', 'Messages': '信息', 'FaceTime': 'FaceTime',
            'Music': '音乐', 'Photos': '照片', 'Preview': '预览',
            'Terminal': '终端', 'iTerm2': 'iTerm2 终端',
            'Activity Monitor': '活动监视器', 'System Preferences': '系统设置',
            'Docker': 'Docker 容器', 'docker': 'Docker 容器',
            'Slack': 'Slack 协作', 'Discord': 'Discord',
            'WeChat': '微信', 'wechat': '微信', 'QQ': 'QQ',
            'DingTalk': '钉钉', 'Lark': '飞书', 'feishu': '飞书',
            'Code Helper': 'VS Code 辅助', 'Electron': 'Electron 应用',
            'node': 'Node.js', 'npm': 'npm 包管理', 'esbuild': 'esbuild 构建',
            'python3': 'Python', 'python': 'Python', 'ruby': 'Ruby',
            'java': 'Java', 'postgres': 'PostgreSQL', 'redis-server': 'Redis',
            'mysqld': 'MySQL', 'mongod': 'MongoDB',
            'Spotify': 'Spotify 音乐', 'zoom.us': 'Zoom 会议',
            'mdworker_shared': 'Spotlight 索引', 'mds_stores': 'Spotlight 存储',
            'cloudd': 'iCloud 同步', 'bird': 'iCloud 后台',
            'launchd': '系统启动守护', 'syslogd': '系统日志',
            'coreaudiod': '核心音频', 'bluetoothd': '蓝牙服务',
            'zsh': 'Zsh 终端', 'bash': 'Bash 终端',
            'caffeinate': '防休眠', 'top': '系统监控(top)',
            'claude': 'Claude AI', 'cursor': 'Cursor 编辑器',
            'macos-perf-monitor': '性能监控(本应用)',
            'PerfMonitor': '性能监控(本应用)',
          };
          const getAppName = (name: string) => appNames[name] || null;
          const filtered = processes.filter((p) => {
            if (!processFilter) return true;
            const q = processFilter.toLowerCase();
            const friendly = getAppName(p.name);
            return p.name.toLowerCase().includes(q)
              || (friendly && friendly.toLowerCase().includes(q))
              || p.pid.toString().includes(q);
          });
          const sorted = [...filtered].sort((a, b) => {
            let cmp = 0;
            if (sortKey === 'pid') cmp = a.pid - b.pid;
            else if (sortKey === 'name') cmp = a.name.localeCompare(b.name);
            else if (sortKey === 'cpu') cmp = a.cpu - b.cpu;
            else if (sortKey === 'mem') cmp = a.mem - b.mem;
            else if (sortKey === 'rss') cmp = a.rss - b.rss;
            return sortAsc ? cmp : -cmp;
          });
          const displayed = processFilter ? sorted : sorted.slice(0, 100);

          const toggleSort = (key: typeof sortKey) => {
            if (sortKey === key) setSortAsc(!sortAsc);
            else { setSortKey(key); setSortAsc(false); }
          };
          const sortIcon = (key: typeof sortKey) => sortKey === key ? (sortAsc ? ' ↑' : ' ↓') : '';

          return (
            <div className="processes-panel">
              <div className="process-toolbar">
                <div className="search-wrapper">
                  <input
                    type="text"
                    className="process-search"
                    placeholder="搜索进程名、应用名或 PID..."
                    value={processFilter}
                    onChange={(e) => setProcessFilter(e.target.value)}
                  />
                  {processFilter && (
                    <button className="search-clear" onClick={() => setProcessFilter('')}>×</button>
                  )}
                </div>
                <span className="process-count">
                  {processFilter ? `${filtered.length} 个结果` : `前 100 / 共 ${processes.length}`}
                </span>
              </div>
              <div className="process-table-scroll">
                <table className="process-table">
                  <thead>
                    <tr>
                      <th className="sortable" onClick={() => toggleSort('pid')}>PID{sortIcon('pid')}</th>
                      <th className="sortable" onClick={() => toggleSort('name')}>进程名{sortIcon('name')}</th>
                      <th className="sortable" onClick={() => toggleSort('cpu')}>CPU%{sortIcon('cpu')}</th>
                      <th className="sortable" onClick={() => toggleSort('mem')}>内存%{sortIcon('mem')}</th>
                      <th className="sortable" onClick={() => toggleSort('rss')}>RSS (MB){sortIcon('rss')}</th>
                      <th>操作</th>
                    </tr>
                  </thead>
                  <tbody>
                  {displayed.map((p) => (
                    <tr key={p.pid}>
                      <td>{p.pid}</td>
                      <td className="process-name">
                        {p.name}
                        {getAppName(p.name) && <span className="app-label">{getAppName(p.name)}</span>}
                      </td>
                      <td className={p.cpu > 50 ? 'high-usage' : ''}>{p.cpu.toFixed(1)}%</td>
                      <td>{p.mem.toFixed(1)}%</td>
                      <td>{(p.rss / 1024 / 1024).toFixed(1)}</td>
                      <td>
                        <button
                          className="kill-btn"
                          onClick={() => {
                            if (confirm(`确定要终止进程 ${p.name} (PID: ${p.pid}) 吗？`)) {
                              invoke('kill_process', { pid: p.pid })
                                .then(() => {
                                  setProcesses((prev) => prev.filter((proc) => proc.pid !== p.pid));
                                })
                                .catch((err: string) => alert(err));
                            }
                          }}
                        >
                          终止
                        </button>
                      </td>
                    </tr>
                  ))}
                  </tbody>
                </table>
              </div>
            </div>
          );
        })()}

        {/* Marks Tab */}
        {activeTab === 'marks' && (
          <div className="marks-panel">
            <p className="marks-desc">在关键节点（如升级、部署、配置变更）打标记，然后到「报告」页对比标记前后的性能差异。</p>
            <div className="mark-input">
              <input
                type="text"
                placeholder="输入标记名称（如：升级前）"
                value={newMarkName}
                onChange={(e) => setNewMarkName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && addMark()}
              />
              <button className="action-btn" onClick={addMark}>添加标记</button>
            </div>
            <div className="marks-list">
              {marks.length === 0 ? (
                <p className="empty-text">暂无时间标记</p>
              ) : (
                marks.map((mark) => (
                  <div key={mark.id} className="mark-item">
                    <div className="mark-info">
                      <div className="mark-name">{mark.name}</div>
                      <div className="mark-time">{new Date(mark.timestamp).toLocaleString()}</div>
                    </div>
                    <button
                      className="kill-btn"
                      onClick={() => {
                        invoke('delete_mark', { id: mark.id })
                          .then(() => setMarks((prev) => prev.filter((m) => m.id !== mark.id)))
                          .catch((err: string) => alert(err));
                      }}
                    >
                      删除
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* Report Tab */}
        {activeTab === 'report' && (
          <div className="report-panel">
            <p className="marks-desc">对比两段连续时间的性能数据。例如：较早 10 分钟 + 最近 10 分钟 = 对比 20 分钟前~10 分钟前 vs 最近 10 分钟的真实采集数据。数据越多越准确。</p>
            <div className="report-config">
              <div className="config-row">
                <label>较早时段:</label>
                <input
                  type="number"
                  value={beforeWindow.minutes}
                  onChange={(e) => setBeforeWindow({ minutes: parseInt(e.target.value) || 0 })}
                />
                <span>分钟</span>
              </div>
              <div className="config-row">
                <label>最近时段:</label>
                <input
                  type="number"
                  value={afterWindow.minutes}
                  onChange={(e) => setAfterWindow({ minutes: parseInt(e.target.value) || 0 })}
                />
                <span>分钟</span>
              </div>
              <button className="action-btn" onClick={generateReport}>生成报告</button>
            </div>

            {report && (
              <div className="report-result">
                <div className="report-summary">
                  <h3>分析结论</h3>
                  <p className={`conclusion ${report.comparison.cpu_better && report.comparison.mem_better ? 'positive' : report.comparison.cpu_better_percent < 0 || report.comparison.mem_better_percent < 0 ? 'negative' : 'neutral'}`}>
                    {report.comparison.conclusion}
                  </p>
                </div>

                <div className="comparison-grid">
                  <div className="comparison-card">
                    <h4>较早时段</h4>
                    <div className="stat-row"><span>平均 CPU</span><strong>{report.before_window.avg_cpu.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>峰值 CPU</span><strong>{report.before_window.peak_cpu.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>平均内存</span><strong>{report.before_window.avg_memory.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>卡顿评分</span><strong className={report.comparison.lag_score_before > 50 ? 'high' : 'low'}>{report.comparison.lag_score_before}</strong></div>
                  </div>
                  <div className="comparison-card">
                    <h4>最近时段</h4>
                    <div className="stat-row"><span>平均 CPU</span><strong>{report.after_window.avg_cpu.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>峰值 CPU</span><strong>{report.after_window.peak_cpu.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>平均内存</span><strong>{report.after_window.avg_memory.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>卡顿评分</span><strong className={report.comparison.lag_score_after > 50 ? 'high' : 'low'}>{report.comparison.lag_score_after}</strong></div>
                  </div>
                  <div className="comparison-card diff">
                    <h4>对比改善</h4>
                    <div className="stat-row"><span>CPU</span><strong className={report.comparison.cpu_better ? 'positive' : 'negative'}>{report.comparison.cpu_better_percent > 0 ? '+' : ''}{report.comparison.cpu_better_percent.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>内存</span><strong className={report.comparison.mem_better ? 'positive' : 'negative'}>{report.comparison.mem_better_percent > 0 ? '+' : ''}{report.comparison.mem_better_percent.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>Swap</span><strong className={report.comparison.swap_better ? 'positive' : 'negative'}>{report.comparison.swap_better_percent > 0 ? '+' : ''}{report.comparison.swap_better_percent.toFixed(1)}%</strong></div>
                    <div className="stat-row"><span>卡顿改善</span><strong className={report.comparison.lag_score_before > report.comparison.lag_score_after ? 'positive' : 'negative'}>{report.comparison.lag_score_before - report.comparison.lag_score_after}</strong></div>
                  </div>
                </div>
              </div>
            )}

            <div className="ranking-section">
              <div className="ranking-header">
                <h3>资源占用排行榜</h3>
                <div className="range-selector">
                  {(['1h', '6h', '24h'] as const).map((r) => (
                    <button key={r} className={`range-btn ${rankingRange === r ? 'active' : ''}`} onClick={() => setRankingRange(r)}>
                      {r === '1h' ? '1小时' : r === '6h' ? '6小时' : '24小时'}
                    </button>
                  ))}
                </div>
              </div>

              {rankings.length === 0 ? (
                <p className="empty-text">暂无数据（进程快照每 30 秒采集一次，请稍等几分钟后查看）</p>
              ) : (
                <>
                  <table className="process-table ranking-table">
                    <thead>
                      <tr>
                        <th>#</th>
                        <th>进程名</th>
                        <th>平均内存%</th>
                        <th>峰值内存 MB</th>
                        <th>平均CPU%</th>
                        <th>峰值CPU%</th>
                        <th>采样数</th>
                      </tr>
                    </thead>
                    <tbody>
                      {rankings.map((r, i) => (
                        <tr key={r.name} className={i < 3 ? 'top-rank' : ''}>
                          <td><span className={`rank-badge rank-${i < 3 ? i + 1 : 'normal'}`}>{i + 1}</span></td>
                          <td className="process-name">{r.name}</td>
                          <td className={r.avg_mem > 5 ? 'high-usage' : ''}>{r.avg_mem}%</td>
                          <td>{r.peak_rss_mb} MB</td>
                          <td className={r.avg_cpu > 30 ? 'high-usage' : ''}>{r.avg_cpu}%</td>
                          <td>{r.peak_cpu}%</td>
                          <td>{r.sample_count}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>

                  <div className="optimization-section">
                    <h3>优化建议</h3>
                    {rankings.slice(0, 5).map((r) => {
                      const tips: string[] = [];
                      if (r.avg_mem > 10) tips.push(`内存占用高（平均 ${r.avg_mem}%），建议定期重启该应用释放内存`);
                      if (r.peak_rss_mb > 2000) tips.push(`内存峰值达 ${r.peak_rss_mb} MB，可能存在内存泄漏，建议关注版本更新`);
                      if (r.avg_cpu > 20) tips.push(`CPU 占用高（平均 ${r.avg_cpu}%），持续高负载会影响续航和发热`);
                      if (r.peak_cpu > 80) tips.push(`CPU 峰值达 ${r.peak_cpu}%，建议检查是否有后台任务或更新卡住`);
                      if (r.sample_count > 50 && r.avg_cpu < 1 && r.avg_mem > 3) tips.push('长期驻留但几乎不用 CPU，考虑是否需要保持运行');
                      if (tips.length === 0) return null;
                      return (
                        <div key={r.name} className="opt-card">
                          <div className="opt-name">{r.name}</div>
                          <ul className="opt-tips">
                            {tips.map((t, j) => <li key={j}>{t}</li>)}
                          </ul>
                        </div>
                      );
                    })}
                    {rankings.slice(0, 5).every((r) => r.avg_mem <= 10 && r.avg_cpu <= 20 && r.peak_cpu <= 80 && r.peak_rss_mb <= 2000) && (
                      <p className="opt-good">系统运行良好，暂无需要特别关注的进程。</p>
                    )}
                  </div>
                </>
              )}
            </div>
          </div>
        )}

        {/* Settings Tab */}
        {activeTab === 'settings' && (
          <div className="settings-panel">
            <div className="settings-group">
              <div className="settings-group-title">通用</div>
              <div className="settings-card">
                <div className="setting-row">
                  <div className="setting-label">
                    <span>开机自启动</span>
                  </div>
                  <label className="toggle-switch">
                    <input
                      type="checkbox"
                      checked={autostart}
                      onChange={(e) => {
                        const val = e.target.checked;
                        invoke('set_autostart', { enabled: val })
                          .then(() => setAutostart(val))
                          .catch((err: string) => alert(err));
                      }}
                    />
                    <span className="toggle-slider"></span>
                  </label>
                </div>
                <div className="setting-row">
                  <div className="setting-label">
                    <span>外观</span>
                  </div>
                  <select
                    className="setting-select"
                    value={theme}
                    onChange={(e) => setTheme(e.target.value as 'auto' | 'light' | 'dark')}
                  >
                    <option value="auto">跟随系统</option>
                    <option value="light">浅色</option>
                    <option value="dark">深色</option>
                  </select>
                </div>
              </div>
            </div>

            <div className="settings-group">
              <div className="settings-group-title">数据采集</div>
              <div className="settings-card">
                <div className="setting-row">
                  <div className="setting-label">
                    <span>采集间隔</span>
                  </div>
                  <div className="setting-stepper">
                    <input
                      type="number"
                      min="1"
                      max="60"
                      value={settings.collectionInterval}
                      onChange={(e) => setSettings({ ...settings, collectionInterval: parseInt(e.target.value) || 3 })}
                    />
                    <span className="setting-unit">秒</span>
                  </div>
                </div>
                <div className="setting-row">
                  <div className="setting-label">
                    <span>数据保留</span>
                  </div>
                  <div className="setting-stepper">
                    <input
                      type="number"
                      min="1"
                      max="365"
                      value={settings.retentionDays}
                      onChange={(e) => setSettings({ ...settings, retentionDays: parseInt(e.target.value) || 30 })}
                    />
                    <span className="setting-unit">天</span>
                  </div>
                </div>
                <div className="setting-row">
                  <div className="setting-label">
                    <span>进程刷新</span>
                  </div>
                  <div className="setting-stepper">
                    <input
                      type="number"
                      min="1"
                      max="300"
                      value={settings.processInterval}
                      onChange={(e) => setSettings({ ...settings, processInterval: parseInt(e.target.value) || 30 })}
                    />
                    <span className="setting-unit">秒</span>
                  </div>
                </div>
              </div>
              <div className="settings-group-footer">修改后需重启应用生效</div>
            </div>

            <div className="settings-group">
              <div className="settings-card">
                <div className="setting-row setting-row-danger" onClick={() => {
                  if (confirm('确定要退出性能监控吗？')) {
                    invoke('quit_app');
                  }
                }}>
                  <span>退出性能监控</span>
                </div>
              </div>
            </div>
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
