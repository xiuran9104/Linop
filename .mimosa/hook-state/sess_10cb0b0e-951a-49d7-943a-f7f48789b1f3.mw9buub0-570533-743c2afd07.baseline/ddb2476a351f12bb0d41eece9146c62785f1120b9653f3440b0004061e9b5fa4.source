//! linop-monitor:通过 SSH exec 周期采集远端资源占用。
//!
//! 对应 navop 的 `resource_view`(服务器监控图表):
//! 单条 POSIX 探针命令读取 /proc 与 df,本地解析后供 UI 画走势。

use std::time::Duration;

use tokio::sync::mpsc;

/// 采样间隔(秒)。探针本身含 `sleep 1` 用于 CPU 差分,故间隔应 ≥ 2s。
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// 单次探针(含远端 sleep 1)允许的最大耗时。
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 每会话保留的样本上限(5s 间隔下约 10 分钟走势)。
pub const MAX_SAMPLES: usize = 120;

/// 一条探针命令:输出依次为 loadavg、MemTotal/MemAvailable、
/// 两次 /proc/stat 首行(间隔 1s,用于 CPU 差分)、根分区 df。
pub const PROBE_COMMAND: &str = concat!(
    "cat /proc/loadavg; ",
    "grep -E '^(MemTotal|MemAvailable):' /proc/meminfo; ",
    "head -n1 /proc/stat; ",
    "sleep 1; ",
    "head -n1 /proc/stat; ",
    "df -kP / 2>/dev/null | tail -n1"
);

/// 一次远端资源采样。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceSample {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    /// 总 CPU 占用百分比 0-100(两次 /proc/stat 差分)。
    pub cpu_percent: f64,
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub disk_total_kb: u64,
    pub disk_used_kb: u64,
}

impl ResourceSample {
    /// 内存占用百分比。
    pub fn mem_percent(&self) -> f64 {
        percent(self.mem_total_kb - self.mem_available_kb, self.mem_total_kb)
    }

    /// 根分区占用百分比。
    pub fn disk_percent(&self) -> f64 {
        percent(self.disk_used_kb, self.disk_total_kb)
    }

    /// 内存已用量(kB)。
    pub fn mem_used_kb(&self) -> u64 {
        self.mem_total_kb.saturating_sub(self.mem_available_kb)
    }
}

fn percent(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 / whole as f64) * 100.0
    }
}

/// 解析探针输出。行序即命令序;多余行忽略,缺失行报错。
pub fn parse_probe(output: &str) -> Result<ResourceSample, String> {
    let lines: Vec<&str> = output.lines().map(str::trim_end).collect();
    let mut iter = lines.iter();

    // 1) loadavg: "0.52 0.58 0.59 1/482 12345"
    let load_line = iter.next().ok_or("缺少 loadavg 行")?;
    let mut load = [0.0f64; 3];
    for (slot, field) in load.iter_mut().zip(load_line.split_whitespace()) {
        *slot = field
            .parse()
            .map_err(|_| format!("loadavg 解析失败:{field:?}"))?;
    }

    // 2-3) meminfo: "MemTotal:  16384000 kB" × 2
    let mut mem_total_kb = None;
    let mut mem_available_kb = None;
    for _ in 0..2 {
        let line = iter.next().ok_or("缺少 meminfo 行")?;
        let (key, value) = parse_meminfo(line)?;
        match key {
            "MemTotal" => mem_total_kb = Some(value),
            "MemAvailable" => mem_available_kb = Some(value),
            other => return Err(format!("意外的 meminfo 项:{other}")),
        }
    }

    // 4-5) /proc/stat 首行 × 2:"cpu user nice system idle iowait irq softirq steal …"
    let stat1 = parse_cpu_line(iter.next().ok_or("缺少第一行 /proc/stat")?)?;
    let stat2 = parse_cpu_line(iter.next().ok_or("缺少第二行 /proc/stat")?)?;
    let total_delta = stat2.0.saturating_sub(stat1.0);
    let idle_delta = stat2.1.saturating_sub(stat1.1);
    let cpu_percent = if total_delta == 0 {
        0.0
    } else {
        100.0 * (1.0 - idle_delta as f64 / total_delta as f64)
    };

    // 6) df -kP: "/dev/sda1 49996160 19998464 27465728 43% /"
    let df_line = iter.next().ok_or("缺少 df 行")?;
    let fields: Vec<&str> = df_line.split_whitespace().collect();
    // 容错:只接受数字形态的块数,避免 bind mount 等输出干扰。
    let nums: Vec<u64> = fields
        .iter()
        .filter_map(|f| f.parse::<u64>().ok())
        .collect();
    if nums.len() < 3 {
        return Err(format!("df 解析失败:{df_line:?}"));
    }
    let (disk_total_kb, disk_used_kb) = (nums[0], nums[1]);

    Ok(ResourceSample {
        load1: load[0],
        load5: load[1],
        load15: load[2],
        cpu_percent,
        mem_total_kb: mem_total_kb.unwrap_or(0),
        mem_available_kb: mem_available_kb.unwrap_or(0),
        disk_total_kb,
        disk_used_kb,
    })
}

/// "MemTotal:  16384000 kB" → ("MemTotal", 16384000)
fn parse_meminfo(line: &str) -> Result<(&str, u64), String> {
    let (key, rest) = line
        .split_once(':')
        .ok_or_else(|| format!("meminfo 行异常:{line:?}"))?;
    let value = rest
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| format!("meminfo 数值异常:{line:?}"))?;
    Ok((key.trim(), value))
}

/// /proc/stat 首行 → (全部字段之和, idle+iowait)
fn parse_cpu_line(line: &str) -> Result<(u64, u64), String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.first().copied() != Some("cpu") {
        return Err(format!("/proc/stat 行异常:{line:?}"));
    }
    let values: Vec<u64> = fields[1..]
        .iter()
        .map(|f| f.parse::<u64>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("/proc/stat 数值异常:{line:?}"))?;
    let total: u64 = values.iter().sum();
    // idle 在第 4 列,iowait 第 5 列(存在时),两者都算"未占用"。
    let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
    Ok((total, idle))
}

/// 单轮采样:执行探针并解析;错误转成消息字符串。
async fn probe_once(ssh: &linop_ssh::Session) -> Result<ResourceSample, String> {
    match tokio::time::timeout(PROBE_TIMEOUT, ssh.exec(PROBE_COMMAND)).await {
        Ok(Ok(output)) => parse_probe(&String::from_utf8_lossy(&output))
            .map_err(|e| format!("探针输出解析失败:{e}")),
        Ok(Err(e)) => Err(format!("探针执行失败:{e}")),
        Err(_) => Err("探针超时".to_string()),
    }
}

/// 采样循环:首个样本立即采集,之后每 SAMPLE_INTERVAL 一次;
/// 每轮结果交给 `emit`。接收端放弃时由调用方 abort 任务停止。
async fn probe_loop(ssh: linop_ssh::Session, mut emit: impl FnMut(Result<ResourceSample, String>)) {
    loop {
        let sample = probe_once(&ssh).await;
        emit(sample);
        tokio::time::sleep(SAMPLE_INTERVAL).await;
    }
}

/// 采样器:后台任务周期执行探针,结果发往通道;停止时 abort 即可。
pub struct Monitor {
    task: tokio::task::JoinHandle<()>,
    rx: mpsc::UnboundedReceiver<Result<ResourceSample, String>>,
}

impl Monitor {
    /// 在 SSH 会话上启动采样;首个样本立即采集,之后按固定间隔。
    pub fn spawn(ssh: linop_ssh::Session) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(probe_loop(ssh, move |sample| {
            let _ = tx.send(sample);
        }));
        Self { task, rx }
    }

    /// 以回调方式启动采样(应用事件循环用):每个样本回调一次,
    /// 返回任务句柄,abort 即停止。
    pub fn spawn_with(
        ssh: linop_ssh::Session,
        on_sample: impl FnMut(Result<ResourceSample, String>) + Send + 'static,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(probe_loop(ssh, on_sample))
    }

    /// 取出当前已就绪的全部采样结果(非阻塞)。
    pub fn drain(&mut self) -> Vec<Result<ResourceSample, String>> {
        let mut out = Vec::new();
        while let Ok(sample) = self.rx.try_recv() {
            out.push(sample);
        }
        out
    }

    /// 等待下一个采样结果(测试与低频轮询用)。
    pub async fn next(&mut self) -> Option<Result<ResourceSample, String>> {
        self.rx.recv().await
    }

    pub fn stop(self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = concat!(
        "0.10 0.20 0.30 1/500 12345\n",
        "MemTotal:       16384000 kB\n",
        "MemAvailable:    8192000 kB\n",
        "cpu  100 20 30 850 10 0 0 0 0 0\n",
        "cpu  150 30 40 1650 20 0 0 0 0 0\n",
        "/dev/sda1 49996160 19998464 27465728 43% /\n",
    );

    #[test]
    fn parses_full_fixture() {
        let sample = parse_probe(FIXTURE).unwrap();
        assert!((sample.load1 - 0.10).abs() < 1e-9);
        assert!((sample.load15 - 0.30).abs() < 1e-9);
        // Δtotal = 1890-1010 = 880,Δidle = 1670-860 = 810 → 100*(1-810/880) ≈ 7.95
        assert!((sample.cpu_percent - (100.0 * 70.0 / 880.0)).abs() < 1e-6);
        assert_eq!(sample.mem_total_kb, 16384000);
        assert_eq!(sample.mem_available_kb, 8192000);
        assert_eq!(sample.mem_percent(), 50.0);
        assert_eq!(sample.disk_total_kb, 49996160);
        assert_eq!(sample.disk_used_kb, 19998464);
        assert!((sample.disk_percent() - 40.0).abs() < 0.01);
    }

    #[test]
    fn idle_only_delta_means_zero_cpu() {
        let fixture = FIXTURE.replace(
            "cpu  150 30 40 1650 20 0 0 0 0 0",
            "cpu  100 20 30 940 10 0 0 0 0 0",
        );
        // 行2 total=1100、idle=950;行1 total=1010、idle=860 → Δtotal=Δidle=90 → CPU 0%。
        let sample = parse_probe(&fixture).unwrap();
        assert!(
            (sample.cpu_percent - 0.0).abs() < 1e-9,
            "增量全在 idle 时 CPU 应为 0"
        );
    }

    #[test]
    fn missing_lines_are_errors() {
        assert!(parse_probe("").is_err());
        assert!(parse_probe("0.1 0.2 0.3 1/1 1").is_err());
        let truncated = FIXTURE.lines().take(5).collect::<Vec<_>>().join("\n");
        assert!(parse_probe(&truncated).is_err());
    }

    #[test]
    fn df_with_extra_numeric_columns_still_parses() {
        // df 输出前端多一列数字(如某些文件系统名含数字)不应错位:
        // 解析器取前三个可解析数字为 total/used/avail,只验证不崩溃且合法。
        let fixture = FIXTURE.replace(
            "/dev/sda1 49996160 19998464 27465728 43% /",
            "tmpfs 123 4 119 4% /dev/shm",
        );
        let sample = parse_probe(&fixture).unwrap();
        assert_eq!(sample.disk_total_kb, 123);
        assert_eq!(sample.disk_used_kb, 4);
    }
}
