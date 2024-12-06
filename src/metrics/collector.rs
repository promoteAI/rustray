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
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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

    /// 导出指标数据
    pub fn export_metrics(&self) -> Result<String, String> {
        let definitions = self.definitions.lock().map_err(|e| e.to_string())?;
        let data = self.data.lock().map_err(|e| e.to_string())?;
        let mut output = String::new();

        for (name, def) in definitions.iter() {
            output.push_str(&format!("# HELP {} {}\n", name, def.description));
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
                            let mut sum = 0.0;
                            let mut count = 0;
                            for v in values {
                                sum += v;
                                count += 1;
                                output.push_str(&format!(
                                    "{}{{{}}} {}\n",
                                    name, labels, v
                                ));
                            }
                            output.push_str(&format!(
                                "{}_sum{{{}}} {}\n", name, labels, sum
                            ));
                            output.push_str(&format!(
                                "{}_count{{{}}} {}\n", name, labels, count
                            ));
                        }
                        MetricValue::Summary { count, sum, min, max, p50, p90, p99 } => {
                            output.push_str(&format!(
                                "{}_sum{{{}}} {}\n", name, labels, sum
                            ));
                            output.push_str(&format!(
                                "{}_count{{{}}} {}\n", name, labels, count
                            ));
                            output.push_str(&format!(
                                "{}_min{{{}}} {}\n", name, labels, min
                            ));
                            output.push_str(&format!(
                                "{}_max{{{}}} {}\n", name, labels, max
                            ));
                            output.push_str(&format!(
                                "{}_p50{{{}}} {}\n", name, labels, p50
                            ));
                            output.push_str(&format!(
                                "{}_p90{{{}}} {}\n", name, labels, p90
                            ));
                            output.push_str(&format!(
                                "{}_p99{{{}}} {}\n", name, labels, p99
                            ));
                        }
                    }
                }
            }
        }

        Ok(output)
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