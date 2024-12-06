//! 指标收集器模块
//! 
//! 本模块实现了一个全面的性能指标收集和监控系统，支持：
//! - 计数器、仪表、直方图等多种指标类型
//! - 标签和维度支持
//! - 聚合和统计功能
//! - 指标导出和持久化
//! - 告警规则配置

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use serde_json::{json, Value};

/// 指标类型
#[derive(Debug, Clone)]
pub enum MetricType {
    /// 计数器（只增不减）
    Counter,
    /// 仪表（可增可减）
    Gauge,
    /// 直方图（数据分布）
    Histogram,
    /// 摘要（百分位数）
    Summary,
}

/// 指标值
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// 单个数值
    Single(f64),
    /// 直方图数据
    Histogram(Vec<f64>),
    /// 摘要数据
    Summary {
        count: u64,
        sum: f64,
        min: f64,
        max: f64,
        p50: f64,
        p90: f64,
        p99: f64,
    },
}

/// 指标定义
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 单位
    pub unit: String,
    /// 标签
    pub labels: HashMap<String, String>,
}

/// 指标数据点
#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    /// 时间戳
    pub timestamp: u64,
    /// 指标值
    pub value: MetricValue,
    /// 标签
    pub labels: HashMap<String, String>,
}

/// 告警规则
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// 规则名称
    pub name: String,
    /// 指标名称
    pub metric_name: String,
    /// 阈值
    pub threshold: f64,
    /// 比较操作符
    pub operator: String,
    /// 持续时间
    pub duration: Duration,
    /// 严重程度
    pub severity: String,
}

/// 指标收集器
pub struct MetricsCollector {
    /// 命名空间
    namespace: String,
    /// 指标定义映射
    definitions: Arc<Mutex<HashMap<String, MetricDefinition>>>,
    /// 指标数据映射
    data: Arc<Mutex<HashMap<String, Vec<MetricDataPoint>>>>,
    /// 告警规则列表
    alert_rules: Arc<Mutex<Vec<AlertRule>>>,
    /// 告警通道
    alert_tx: mpsc::Sender<Alert>,
}

/// 告警信息
#[derive(Debug, Clone)]
pub struct Alert {
    /// 告警名称
    pub name: String,
    /// 指标名称
    pub metric_name: String,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 严重程度
    pub severity: String,
    /// 触发时间
    pub timestamp: u64,
    /// 标签
    pub labels: HashMap<String, String>,
}

impl MetricsCollector {
    /// 创建新的指标收集器
    pub fn new(namespace: String) -> Self {
        let (alert_tx, _) = mpsc::channel(100);
        Self {
            namespace,
            definitions: Arc::new(Mutex::new(HashMap::new())),
            data: Arc::new(Mutex::new(HashMap::new())),
            alert_rules: Arc::new(Mutex::new(Vec::new())),
            alert_tx,
        }
    }

    /// 注册新指标
    pub fn register_metric(
        &self,
        name: &str,
        metric_type: MetricType,
        description: &str,
        unit: &str,
        labels: HashMap<String, String>,
    ) -> Result<(), String> {
        let metric_name = format!("{}.{}", self.namespace, name);
        let mut definitions = self.definitions.lock().map_err(|e| e.to_string())?;

        let definition = MetricDefinition {
            name: metric_name.clone(),
            metric_type,
            description: description.to_string(),
            unit: unit.to_string(),
            labels,
        };

        definitions.insert(metric_name.clone(), definition);
        self.data.lock().map_err(|e| e.to_string())?
            .insert(metric_name, Vec::new());

        info!("Registered metric: {}", name);
        Ok(())
    }

    /// 增加计数器值
    pub fn increment_counter(&self, name: &str, value: u64) -> Result<(), String> {
        self.record_value(name, MetricValue::Single(value as f64))
    }

    /// 设置仪表值
    pub fn set_gauge(&self, name: &str, value: f64) -> Result<(), String> {
        self.record_value(name, MetricValue::Single(value))
    }

    /// 记录直方图数据
    pub fn record_histogram(&self, name: &str, value: f64) -> Result<(), String> {
        let metric_name = format!("{}.{}", self.namespace, name);
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        
        if let Some(points) = data.get_mut(&metric_name) {
            if let Some(last_point) = points.last() {
                if let MetricValue::Histogram(mut values) = last_point.value.clone() {
                    values.push(value);
                    points.push(MetricDataPoint {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        value: MetricValue::Histogram(values),
                        labels: last_point.labels.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 记录摘要数据
    pub fn record_summary(&self, name: &str, value: f64) -> Result<(), String> {
        let metric_name = format!("{}.{}", self.namespace, name);
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        
        if let Some(points) = data.get_mut(&metric_name) {
            if let Some(last_point) = points.last() {
                if let MetricValue::Summary { count, sum, min, max, .. } = last_point.value {
                    let new_count = count + 1;
                    let new_sum = sum + value;
                    let new_min = min.min(value);
                    let new_max = max.max(value);
                    
                    // 简单计算百分位数（实际应该使用更复杂的算法）
                    points.push(MetricDataPoint {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        value: MetricValue::Summary {
                            count: new_count,
                            sum: new_sum,
                            min: new_min,
                            max: new_max,
                            p50: value, // 简化处理
                            p90: value,
                            p99: value,
                        },
                        labels: last_point.labels.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 添加告警规则
    pub fn add_alert_rule(
        &self,
        name: &str,
        metric_name: &str,
        threshold: f64,
        operator: &str,
        duration: Duration,
        severity: &str,
    ) -> Result<(), String> {
        let rule = AlertRule {
            name: name.to_string(),
            metric_name: format!("{}.{}", self.namespace, metric_name),
            threshold,
            operator: operator.to_string(),
            duration,
            severity: severity.to_string(),
        };

        self.alert_rules.lock().map_err(|e| e.to_string())?
            .push(rule);

        info!("Added alert rule: {}", name);
        Ok(())
    }

    /// 检查告警规则
    pub fn check_alerts(&self) -> Result<(), String> {
        let rules = self.alert_rules.lock().map_err(|e| e.to_string())?;
        let data = self.data.lock().map_err(|e| e.to_string())?;

        for rule in rules.iter() {
            if let Some(points) = data.get(&rule.metric_name) {
                if let Some(latest) = points.last() {
                    if let MetricValue::Single(value) = latest.value {
                        let triggered = match rule.operator.as_str() {
                            ">" => value > rule.threshold,
                            "<" => value < rule.threshold,
                            ">=" => value >= rule.threshold,
                            "<=" => value <= rule.threshold,
                            "==" => (value - rule.threshold).abs() < f64::EPSILON,
                            _ => false,
                        };

                        if triggered {
                            let alert = Alert {
                                name: rule.name.clone(),
                                metric_name: rule.metric_name.clone(),
                                current_value: value,
                                threshold: rule.threshold,
                                severity: rule.severity.clone(),
                                timestamp: latest.timestamp,
                                labels: latest.labels.clone(),
                            };

                            self.alert_tx.try_send(alert)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 添加指标聚合
    pub fn aggregate_metrics(&self, window: Duration) -> Result<AggregatedMetrics, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window_start = now - window.as_secs();

        let mut aggregated = AggregatedMetrics::default();

        for (name, points) in data.iter() {
            // 只处理窗口内的数据点
            let window_points: Vec<_> = points.iter()
                .filter(|p| p.timestamp >= window_start)
                .collect();

            if window_points.is_empty() {
                continue;
            }

            match &window_points[0].value {
                MetricValue::Single(_) => {
                    let values: Vec<f64> = window_points.iter()
                        .filter_map(|p| {
                            if let MetricValue::Single(v) = p.value {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !values.is_empty() {
                        let stats = calculate_statistics(&values);
                        aggregated.metrics.insert(name.clone(), stats);
                    }
                }
                MetricValue::Histogram(values) => {
                    let mut all_values = Vec::new();
                    for p in window_points {
                        if let MetricValue::Histogram(v) = &p.value {
                            all_values.extend(v);
                        }
                    }
                    if !all_values.is_empty() {
                        let stats = calculate_statistics(&all_values);
                        aggregated.histograms.insert(name.clone(), stats);
                    }
                }
                MetricValue::Summary { .. } => {
                    let summaries: Vec<_> = window_points.iter()
                        .filter_map(|p| {
                            if let MetricValue::Summary { count, sum, min, max, p50, p90, p99 } = p.value {
                                Some(SummaryStats {
                                    count,
                                    sum,
                                    min,
                                    max,
                                    p50,
                                    p90,
                                    p99,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    if !summaries.is_empty() {
                        aggregated.summaries.insert(name.clone(), summaries);
                    }
                }
            }
        }

        Ok(aggregated)
    }

    /// 添加异常检测
    pub fn detect_anomalies(&self, window: Duration, threshold: f64) -> Result<Vec<Anomaly>, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window_start = now - window.as_secs();
        let mut anomalies = Vec::new();

        for (name, points) in data.iter() {
            let window_points: Vec<_> = points.iter()
                .filter(|p| p.timestamp >= window_start)
                .collect();

            if window_points.len() < 2 {
                continue;
            }

            // 计算移动平均和标准差
            let values: Vec<f64> = window_points.iter()
                .filter_map(|p| {
                    match p.value {
                        MetricValue::Single(v) => Some(v),
                        _ => None,
                    }
                })
                .collect();

            if values.len() < 2 {
                continue;
            }

            let stats = calculate_statistics(&values);
            
            // 检测异常值
            for (i, &value) in values.iter().enumerate() {
                let z_score = (value - stats.mean).abs() / stats.stddev;
                if z_score > threshold {
                    anomalies.push(Anomaly {
                        metric_name: name.clone(),
                        timestamp: window_points[i].timestamp,
                        value,
                        expected_value: stats.mean,
                        deviation: z_score,
                    });
                }
            }
        }

        Ok(anomalies)
    }

    /// 添加指标数据清理
    pub fn cleanup_old_data(&self, retention: Duration) -> Result<usize, String> {
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cutoff = now - retention.as_secs();
        let mut cleaned = 0;

        for points in data.values_mut() {
            let original_len = points.len();
            points.retain(|p| p.timestamp >= cutoff);
            cleaned += original_len - points.len();
        }

        // 清理空的指标
        data.retain(|_, points| !points.is_empty());

        Ok(cleaned)
    }

    /// 添加指标导出格式化
    pub fn format_metrics(&self, format: MetricFormat) -> Result<String, String> {
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let definitions = self.definitions.lock().map_err(|e| e.to_string())?;

        match format {
            MetricFormat::Prometheus => {
        let mut output = String::new();

        for (name, def) in definitions.iter() {
                    // 添加HELP注释
            output.push_str(&format!("# HELP {} {}\n", name, def.description));
                    // 添加TYPE注释
            output.push_str(&format!("# TYPE {} {:?}\n", name, def.metric_type));

            if let Some(points) = data.get(name) {
                if let Some(latest) = points.last() {
                    let labels = latest.labels.iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(",");

                    match &latest.value {
                        MetricValue::Single(v) => {
                            output.push_str(&format!("{}{{{}}} {}\n", name, labels, v));
                        }
                        MetricValue::Histogram(values) => {
                                    let mut buckets = vec![0.0, 0.5, 1.0, 2.5, 5.0, 10.0];
                                    let mut counts = vec![0; buckets.len() + 1];
                            let mut sum = 0.0;

                                    for &v in values {
                                sum += v;
                                        for (i, &bucket) in buckets.iter().enumerate() {
                                            if v <= bucket {
                                                counts[i] += 1;
                                            }
                                        }
                                        counts[buckets.len()] += 1;
                                    }

                                    for (i, &bucket) in buckets.iter().enumerate() {
                                output.push_str(&format!(
                                            "{}{{{}},le=\"{}\"}} {}\n",
                                            name, labels, bucket, counts[i]
                                ));
                            }
                            output.push_str(&format!(
                                        "{}{{{}},le=\"+Inf\"}} {}\n",
                                        name, labels, counts[buckets.len()]
                                    ));
                                    output.push_str(&format!("{}_sum{{{}}} {}\n", name, labels, sum));
                                    output.push_str(&format!("{}_count{{{}}} {}\n", name, labels, values.len()));
                                }
                                MetricValue::Summary { count, sum, min, max, p50, p90, p99 } => {
                                    output.push_str(&format!("{}_sum{{{}}} {}\n", name, labels, sum));
                                    output.push_str(&format!("{}_count{{{}}} {}\n", name, labels, count));
                                    output.push_str(&format!("{}_min{{{}}} {}\n", name, labels, min));
                                    output.push_str(&format!("{}_max{{{}}} {}\n", name, labels, max));
                                    output.push_str(&format!("{}{{{}},quantile=\"0.5\"}} {}\n", name, labels, p50));
                                    output.push_str(&format!("{}{{{}},quantile=\"0.9\"}} {}\n", name, labels, p90));
                                    output.push_str(&format!("{}{{{}},quantile=\"0.99\"}} {}\n", name, labels, p99));
                                }
                            }
                        }
                    }
                }

                Ok(output)
            }
            MetricFormat::JSON => {
                let mut metrics = serde_json::Map::new();
                
                for (name, points) in data.iter() {
                    if let Some(latest) = points.last() {
                        let mut metric = serde_json::Map::new();
                        metric.insert("timestamp".to_string(), json!(latest.timestamp));
                        metric.insert("labels".to_string(), json!(latest.labels));
                        
                        match &latest.value {
                            MetricValue::Single(v) => {
                                metric.insert("value".to_string(), json!(v));
                            }
                            MetricValue::Histogram(values) => {
                                metric.insert("values".to_string(), json!(values));
                        }
                        MetricValue::Summary { count, sum, min, max, p50, p90, p99 } => {
                                let summary = json!({
                                    "count": count,
                                    "sum": sum,
                                    "min": min,
                                    "max": max,
                                    "p50": p50,
                                    "p90": p90,
                                    "p99": p99,
                                });
                                metric.insert("summary".to_string(), summary);
                            }
                        }
                        
                        metrics.insert(name.clone(), serde_json::Value::Object(metric));
                    }
                }

                serde_json::to_string_pretty(&metrics)
                    .map_err(|e| format!("Failed to serialize metrics: {}", e))
            }
        }
    }

    // 私有辅助方法

    /// 记录指标值
    fn record_value(&self, name: &str, value: MetricValue) -> Result<(), String> {
        let metric_name = format!("{}.{}", self.namespace, name);
        let mut data = self.data.lock().map_err(|e| e.to_string())?;
        
        if let Some(points) = data.get_mut(&metric_name) {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let labels = self.definitions.lock().map_err(|e| e.to_string())?
                .get(&metric_name)
                .map(|def| def.labels.clone())
                .unwrap_or_default();

            points.push(MetricDataPoint {
                timestamp,
                value,
                labels,
            });

            // 检查告警
            self.check_alerts()?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum MetricFormat {
    Prometheus,
    JSON,
}

#[derive(Debug, Default)]
pub struct AggregatedMetrics {
    pub metrics: HashMap<String, Statistics>,
    pub histograms: HashMap<String, Statistics>,
    pub summaries: HashMap<String, Vec<SummaryStats>>,
}

#[derive(Debug, Clone)]
pub struct Statistics {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub stddev: f64,
}

#[derive(Debug, Clone)]
pub struct SummaryStats {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

#[derive(Debug)]
pub struct Anomaly {
    pub metric_name: String,
    pub timestamp: u64,
    pub value: f64,
    pub expected_value: f64,
    pub deviation: f64,
}

fn calculate_statistics(values: &[f64]) -> Statistics {
    let count = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    let variance = values.iter()
        .map(|x| {
            let diff = x - mean;
            diff * diff
        })
        .sum::<f64>() / count as f64;
    
    let stddev = variance.sqrt();

    Statistics {
        count,
        sum,
        mean,
        min,
        max,
        stddev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new("test".to_string());
        assert!(collector.definitions.lock().unwrap().is_empty());
        assert!(collector.data.lock().unwrap().is_empty());
    }

    #[test]
    fn test_metric_registration() {
        let collector = MetricsCollector::new("test".to_string());
        let result = collector.register_metric(
            "requests_total",
            MetricType::Counter,
            "Total number of requests",
            "requests",
            HashMap::new(),
        );
        
        assert!(result.is_ok());
        assert_eq!(collector.definitions.lock().unwrap().len(), 1);
        assert_eq!(collector.data.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_counter_increment() {
        let collector = MetricsCollector::new("test".to_string());
        collector.register_metric(
            "requests_total",
            MetricType::Counter,
            "Total number of requests",
            "requests",
            HashMap::new(),
        ).unwrap();

        let result = collector.increment_counter("requests_total", 1);
        assert!(result.is_ok());

        let data = collector.data.lock().unwrap();
        let points = data.get("test.requests_total").unwrap();
        assert_eq!(points.len(), 1);
        
        if let MetricValue::Single(value) = points[0].value {
            assert_eq!(value, 1.0);
        } else {
            panic!("Unexpected metric value type");
        }
    }

    #[test]
    fn test_alert_rule() {
        let collector = MetricsCollector::new("test".to_string());
        collector.register_metric(
            "cpu_usage",
            MetricType::Gauge,
            "CPU usage percentage",
            "percent",
            HashMap::new(),
        ).unwrap();

        let result = collector.add_alert_rule(
            "high_cpu",
            "cpu_usage",
            90.0,
            ">",
            Duration::from_secs(60),
            "critical",
        );
        
        assert!(result.is_ok());
        assert_eq!(collector.alert_rules.lock().unwrap().len(), 1);
    }
} 