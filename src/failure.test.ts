import {it,expect} from 'vitest';
import {describeFailure} from './failure';
it('names an upstream provider failure and keeps its reason',()=>{
 const failure=describeFailure('The model call failed: ProviderResponseError: status 503 Service Unavailable: {"error":{"code":"model_not_found","message":"No available channel for model deepseek-v4.1-flash under group default","type":"new_api_error"}}');
 expect(failure.title).toBe('供应商错误');expect(failure.detail).toBe('HTTP 503 · No available channel for model deepseek-v4.1-flash under group default');
});
it('keeps a rejection without an inner message readable',()=>{
 const failure=describeFailure('ProviderResponseError: status 400 Bad Request: {"error":{"code":"400001","type":"invalid_request_error"}}');
 expect(failure.title).toBe('供应商错误');expect(failure.detail).toContain('HTTP 400');expect(failure.detail).toContain('400001');
});
it('does not label an unrelated runtime failure as a provider error',()=>{
 const failure=describeFailure('workspace permission denied (os error 5)');
 expect(failure.title).toBe('任务失败');expect(failure.detail).toBe('workspace permission denied (os error 5)');
});
