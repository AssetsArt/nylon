package sdk

type ResponseFilter struct {
	Request  *Request
	response *Response
}

func (r *ResponseFilter) Headers() map[string]string {
	return r.response.Headers
}

func (r *ResponseFilter) SetHeader(key, value string) {
	r.response.SetHeader(key, value)
}

func (r *ResponseFilter) RemoveHeader(key string) {
	r.response.RemoveHeader(key)
}
