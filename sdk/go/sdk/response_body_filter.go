package sdk

import "encoding/json"

type ResponseBodyFilter struct {
	http_ctx *HttpContext
}

func (r *ResponseBodyFilter) ToBytes() []byte {
	return r.http_ctx.ToBytes()
}

func (r *ResponseBodyFilter) BodyJSON() (map[string]any, error) {
	jsonData := map[string]any{}
	err := json.Unmarshal(r.http_ctx.Response.Body, &jsonData)
	return jsonData, err
}

func (r *ResponseBodyFilter) BodyRaw() []byte {
	return r.http_ctx.Response.Body
}

func (r *ResponseBodyFilter) SetBodyRaw(body []byte) {
	r.http_ctx.Response.Body = body
}

func (r *ResponseBodyFilter) SetBodyJSON(jsonData map[string]any) error {
	jsonDataBytes, err := json.Marshal(jsonData)
	if err != nil {
		return err
	}
	r.http_ctx.Response.Body = jsonDataBytes
	return nil
}
