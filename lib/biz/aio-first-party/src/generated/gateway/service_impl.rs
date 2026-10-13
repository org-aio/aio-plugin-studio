// AIO 根据元数据生成的 Service 实现起点；内容修改后自动转为人工所有。
use super::model::{EndpointFuture, EndpointRequest};
use super::service::GatewayService;
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

#[dill::component]
#[dill::interface(dyn GatewayService)]
#[dill::scope(dill::Singleton)]
#[derive(Debug, Default)]
pub(crate) struct GatewayServiceImpl;

impl GatewayService for GatewayServiceImpl {
    fn get_status(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Edge Gateway Status尚未实现")
        })
    }

    fn get_example(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Gateway Example Plan尚未实现")
        })
    }

    fn post_run(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Run Gateway Plan尚未实现")
        })
    }

    fn get_assets(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Callable Edge Assets尚未实现")
        })
    }

    fn post_assets_weather_current(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Weather Current Asset尚未实现")
        })
    }

    fn post_assets_weather_history(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move { sync_weather_history(&request.body).await })
    }

    fn get_assets_usage(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Edge Asset Usage尚未实现")
        })
    }

    fn get_routes(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Managed API Routes尚未实现")
        })
    }

    fn post_route(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Managed API Route尚未实现")
        })
    }

    fn post_ui_route(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("网关路由页面操作尚未实现")
        })
    }

    fn get_flows(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Gateway Flows尚未实现")
        })
    }

    fn post_flow(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        Box::pin(async move {
            let _ = request;
            bail!("Save Gateway Flow尚未实现")
        })
    }
}

// ---------------------------------------------------------------------------
// 历史天气资产：移植自 boxun-app backend-topcoat 的 weather.rs
// ---------------------------------------------------------------------------

/// 解析后的单日历史天气。
#[derive(Clone, Debug, PartialEq)]
struct WeatherDay {
    date: String,
    week: Option<String>,
    high: f64,
    low: f64,
    morning: Option<String>,
    afternoon: Option<String>,
    wind: Option<String>,
    aqi: i32,
}

const WEATHER_ENDPOINT: &str = "https://tianqi.2345.com/Pc/GetHistory";
const WEATHER_HISTORY_TABLE: &str = "biz_edge_gateway_weather_history";
const DATABASE_URL_ENV: &str = "AZ_AIO_DATABASE_URL";

async fn sync_weather_history(body: &Value) -> Result<Value> {
    let area_id = body
        .get("area_id")
        .or_else(|| body.get("areaId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("area_id 不能为空")?;
    let year = read_integer(body, "year").context("year 不能为空")?;
    ensure!((1900..=2100).contains(&year), "year 不合法: {year}");
    let month = read_integer(body, "month").context("month 不能为空")?;
    ensure!((1..=12).contains(&month), "month 不合法: {month}");

    let html = fetch_weather_month(area_id, year, month as u32)
        .await
        .with_context(|| format!("抓取 {year}-{month:02} 历史天气失败"))?;
    let days = parse_weather_history(&html).context("解析历史天气表格失败")?;
    let pool = weather_history_pool().await?;
    let mut transaction = pool.begin().await.context("开启历史天气事务失败")?;
    let mut accumulated: f64 = 0.0;
    let mut rows = Vec::with_capacity(days.len());
    for day in &days {
        let high = day.high;
        let low = day.low;
        accumulated = (accumulated + (high + low) / 2.0).round();
        sqlx::query(
            "DELETE FROM biz_edge_gateway_weather_history WHERE area_id = $1 AND weather_date = $2",
        )
        .bind(area_id)
        .bind(&day.date)
        .execute(&mut *transaction)
        .await
        .with_context(|| format!("清理 {} 旧天气记录失败", day.date))?;
        sqlx::query(
            "INSERT INTO biz_edge_gateway_weather_history \
             (id, area_id, weather_date, week, maximum_temperature, lowest_temperature, \
              morning, afternoon, wind_direction, air_quality_index, accumulated_temperature, updated_at_ms) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(az_plugin_core::database::new_uuid_id())
        .bind(area_id)
        .bind(&day.date)
        .bind(day.week.clone().unwrap_or_default())
        .bind(high)
        .bind(low)
        .bind(day.morning.clone().unwrap_or_default())
        .bind(day.afternoon.clone().unwrap_or_default())
        .bind(day.wind.clone().unwrap_or_default())
        .bind(day.aqi)
        .bind(accumulated)
        .bind(az_plugin_core::timestamp_ms())
        .execute(&mut *transaction)
        .await
        .with_context(|| format!("写入 {} 天气记录失败", day.date))?;
        rows.push(json!({
            "weatherDate": day.date,
            "week": day.week,
            "maximumTemperature": high,
            "lowestTemperature": low,
            "morning": day.morning,
            "afternoon": day.afternoon,
            "windDirection": day.wind,
            "airQualityIndex": day.aqi,
            "accumulatedTemperature": accumulated,
        }));
    }
    transaction.commit().await.context("提交历史天气事务失败")?;
    Ok(json!({
        "areaId": area_id,
        "year": year,
        "month": month,
        "days": days.len(),
        "rows": rows,
    }))
}

fn read_integer(body: &Value, name: &str) -> Option<i32> {
    body.get(name)
        .and_then(|value| value.as_i64().map(|value| value as i32).or_else(|| {
            value.as_str().and_then(|value| value.trim().parse::<i32>().ok())
        }))
}

async fn weather_history_pool() -> Result<sqlx::PgPool> {
    let database_url = std::env::var(DATABASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("未配置 {DATABASE_URL_ENV}，历史天气无法持久化"))?;
    let database_url =
        az_plugin_core::verify_database_url(&database_url).context("历史天气数据库连接串无效")?;
    sqlx::PgPool::connect(database_url)
        .await
        .with_context(|| format!("连接历史天气数据库失败: {WEATHER_HISTORY_TABLE}"))
}

async fn fetch_weather_month(area_id: &str, year: i32, month: u32) -> Result<String> {
    let query = format!(
        "areaInfo[areaId]={area_id}&areaInfo[areaType]=2&date[year]={year}&date[month]={month}"
    );
    let endpoint = format!("{WEATHER_ENDPOINT}?{query}");
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("创建天气请求客户端失败")?;
    let response = client
        .get(&endpoint)
        .header("accept", "application/json, text/javascript, */*; q=0.01")
        .header("accept-language", "zh-CN,zh;q=0.9")
        .header(
            "referer",
            format!("https://tianqi.2345.com/wea_history/{area_id}.htm"),
        )
        .header(
            "user-agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/111.0.0.0 Safari/537.36",
        )
        .header("x-requested-with", "XMLHttpRequest")
        .send()
        .await
        .context("天气出站请求失败")?;
    let status = response.status();
    let bytes = response.bytes().await.context("读取天气响应失败")?;
    let text = decode_weather_body(&bytes);
    if !status.is_success() {
        bail!("天气服务返回 HTTP {status}：{}", text.chars().take(200).collect::<String>());
    }
    if text.contains("抱歉，暂无") {
        return Ok(String::new());
    }
    let payload: Value = serde_json::from_str(&text).context("天气响应不是 JSON")?;
    Ok(payload
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned())
}

/// 天气响应正文按 UTF-8 优先、GBK 兜底解码。
fn decode_weather_body(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .unwrap_or_else(|_| encoding_rs::GBK.decode(bytes).0.into_owned())
}

fn parse_weather_history(html: &str) -> Result<Vec<WeatherDay>> {
    let table = html
        .split_once("<tbody")
        .and_then(|(_, rest)| rest.split_once("</tbody>").map(|(body, _)| body))
        .unwrap_or(html);
    let mut days = Vec::new();
    for row in table.split("<tr") {
        let cells = row
            .split("<td")
            .skip(1)
            .map(|cell| {
                let value = cell
                    .split_once('>')
                    .map(|(_, value)| value)
                    .unwrap_or(cell)
                    .split_once("</td>")
                    .map(|(value, _)| value)
                    .unwrap_or(cell);
                value
                    .replace(['\n', '\r'], "")
                    .replace("&nbsp;", " ")
                    .trim()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        if cells.len() < 4 {
            continue;
        }
        let mut date_parts = cells[0].split_whitespace();
        let date_text = date_parts.next().unwrap_or_default();
        if !valid_weather_date(date_text) {
            continue;
        }
        let high = weather_number(&cells[1]);
        let low = weather_number(&cells[2]);
        days.push(WeatherDay {
            date: date_text.to_owned(),
            week: date_parts.next().map(str::to_owned),
            high,
            low,
            morning: cells.get(3).cloned(),
            afternoon: None,
            wind: cells.get(4).cloned(),
            aqi: cells
                .get(5)
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or(0),
        });
    }
    Ok(days)
}

fn valid_weather_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 {
        return false;
    }
    let Ok(year) = parts[0].parse::<i32>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

fn weather_number(value: &str) -> f64 {
    value
        .chars()
        .filter(|character| character.is_ascii_digit() || *character == '-' || *character == '.')
        .collect::<String>()
        .parse()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_history_table() {
        let html = "<tbody><tr><td>2026-01-01 周三</td><td>10℃</td><td>-2℃</td><td>晴</td><td>东风 2级</td><td>50 优</td></tr></tbody>";
        let rows = parse_weather_history(html).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].high, 10.0);
        assert_eq!(rows[0].low, -2.0);
        assert_eq!(rows[0].aqi, 50);
        assert_eq!(rows[0].wind.as_deref(), Some("东风 2级"));
    }

    #[test]
    fn rejects_invalid_weather_dates() {
        assert!(valid_weather_date("2026-01-31"));
        assert!(valid_weather_date("2024-02-29"));
        assert!(!valid_weather_date("2026-02-29"));
        assert!(!valid_weather_date("2026-13-01"));
    }
}
