package sdk

import (
	"encoding/json"
	"fmt"
	"strconv"
)

type HttpContext struct {
	// Request  Request
	Response Response
}

// Response
type Response struct {
	_ctx *NylonHttpPluginCtx
}

// Builder
func (r *Response) SetHeader(key, value string) {
	type SetResponseHeaderData struct {
		Key   string `json:"key"`
		Value string `json:"value"`
	}
	data := SetResponseHeaderData{
		Key:   key,
		Value: value,
	}
	jsonData, err := json.Marshal(data)
	if err != nil {
		fmt.Println("[NylonPlugin] SetResponseHeader JSON marshal error:", err)
		return
	}
	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseHeader, jsonData)
}

func (r *Response) RemoveHeader(key string) {
	// delete(r.Headers, key)
	RequestMethod(r._ctx.sessionID, NylonMethodRemoveResponseHeader, []byte(key))
}

func (r *Response) SetStatus(status int) {
	RequestMethod(r._ctx.sessionID, NylonMethodSetResponseStatus, []byte(strconv.Itoa(status)))
}

func (r *Response) BodyRaw(body []byte) {
	panic("not implemented")
}

func (r *Response) BodyJSON(v any) *Response {
	r.SetHeader("Content-Type", "application/json")
	b, _ := json.Marshal(v)
	r.BodyRaw(b)
	return r
}

func (r *Response) BodyText(s string) *Response {
	r.SetHeader("Content-Type", "text/plain; charset=utf-8")
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) BodyHTML(s string) *Response {
	r.SetHeader("Content-Type", "text/html; charset=utf-8")
	r.BodyRaw([]byte(s))
	return r
}

func (r *Response) Redirect(url string, code ...int) *Response {
	status := 302 // default
	if len(code) > 0 {
		status = code[0]
	}
	r.SetStatus(status)
	r.SetHeader("Location", url)
	r.BodyRaw([]byte{})
	return r
}
