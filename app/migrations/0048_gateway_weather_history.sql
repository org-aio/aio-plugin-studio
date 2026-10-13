-- 边缘网关历史天气资产：注册约定接口并建立持久化表。

CREATE FUNCTION engine_gateway_weather_history_endpoints(definition JSONB)
RETURNS JSONB
LANGUAGE plpgsql
IMMUTABLE
AS $$
DECLARE
    endpoint JSONB := '{
      "id": "81cb2b87-6c40-b624-25f9-b59af212b621",
      "title": "Weather History Asset",
      "description": "按月同步与查询 2345 天气历史数据，持久化到 biz_edge_gateway_weather_history。",
      "state": {"kind": "known"},
      "method": "POST",
      "path": "/api/edge-gateway/assets/weather/history",
      "inputs": [
        {"name": "area_id", "title": "地区编号", "location": "body", "value_type": {"kind": "text"}, "required": true},
        {"name": "year", "title": "年份", "location": "body", "value_type": {"kind": "integer"}, "required": true},
        {"name": "month", "title": "月份", "location": "body", "value_type": {"kind": "integer"}, "required": true}
      ],
      "outputs": [
        {"name": "days", "title": "解析天数", "value_type": {"kind": "integer"}},
        {"name": "rows", "title": "历史天气记录", "value_type": {"kind": "any"}}
      ]
    }'::jsonb;
    page_endpoints JSONB;
    exists_endpoint BOOLEAN;
BEGIN
    SELECT page.value -> 'endpoints', EXISTS(
        SELECT 1
        FROM jsonb_array_elements(COALESCE(page.value -> 'endpoints', '[]'::jsonb)) AS item(value)
        WHERE item.value ->> 'path' = endpoint ->> 'path'
          AND item.value ->> 'method' = endpoint ->> 'method'
    )
    INTO page_endpoints, exists_endpoint
    FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb)) AS page(value)
    WHERE page.value ->> 'name' = 'gateway'
    LIMIT 1;

    IF page_endpoints IS NULL OR exists_endpoint THEN
        RETURN definition;
    END IF;

    RETURN (
        SELECT jsonb_set(definition, '{pages}', pages)
        FROM (
            SELECT jsonb_agg(
                CASE
                    WHEN page.value ->> 'name' = 'gateway'
                        THEN jsonb_set(page.value, '{endpoints}', page_endpoints || jsonb_build_array(endpoint))
                    ELSE page.value
                END
                ORDER BY page.ordinality
            ) AS pages
            FROM jsonb_array_elements(COALESCE(definition -> 'pages', '[]'::jsonb)) AS page(value)
                WITH ORDINALITY
        ) AS aggregated
    );
END;
$$;

UPDATE engine_program_drafts
SET definition = engine_gateway_weather_history_endpoints(definition),
    version = version + 1,
    updated_at_ms = (EXTRACT(EPOCH FROM clock_timestamp()) * 1000)::BIGINT
WHERE definition ->> 'name' = 'aio-first-party';

DROP TRIGGER engine_program_revisions_immutable ON engine_program_revisions;

UPDATE engine_program_revisions
SET definition = engine_gateway_weather_history_endpoints(definition),
    content_hash = 'gateway-weather-history:' || md5(
        engine_gateway_weather_history_endpoints(definition)::TEXT
    )
WHERE definition ->> 'name' = 'aio-first-party';

CREATE TRIGGER engine_program_revisions_immutable
BEFORE UPDATE OR DELETE ON engine_program_revisions
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP TRIGGER engine_program_images_immutable ON engine_program_images;

DELETE FROM engine_program_images;

CREATE TRIGGER engine_program_images_immutable
BEFORE UPDATE OR DELETE ON engine_program_images
FOR EACH ROW EXECUTE FUNCTION engine_reject_immutable_program_row();

DROP FUNCTION engine_gateway_weather_history_endpoints(JSONB);

CREATE TABLE biz_edge_gateway_weather_history (
    id TEXT PRIMARY KEY,
    area_id TEXT NOT NULL,
    weather_date DATE NOT NULL,
    week TEXT NOT NULL,
    maximum_temperature REAL NOT NULL,
    lowest_temperature REAL NOT NULL,
    morning TEXT NOT NULL,
    afternoon TEXT NOT NULL,
    wind_direction TEXT NOT NULL,
    air_quality_index INTEGER NOT NULL,
    accumulated_temperature REAL NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    UNIQUE (area_id, weather_date)
);
CREATE INDEX biz_edge_gateway_weather_history_area_date_idx
    ON biz_edge_gateway_weather_history (area_id, weather_date);
