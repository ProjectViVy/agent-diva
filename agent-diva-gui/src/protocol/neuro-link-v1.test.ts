import Ajv2020 from 'ajv/dist/2020'
import { describe, expect, it } from 'vitest'

import protocolSchema from '../../../schemas/neuro-link/v1/protocol.schema.json'
import forgedIdentity from '../../../schemas/neuro-link/v1/fixtures/invalid/forged-identity.json'
import badExtension from '../../../schemas/neuro-link/v1/fixtures/invalid/bad-extension.json'
import legacyPipe from '../../../schemas/neuro-link/v1/fixtures/invalid/legacy-pipe.json'
import malformedContent from '../../../schemas/neuro-link/v1/fixtures/invalid/malformed-content.json'
import protocolVersionMismatch from '../../../schemas/neuro-link/v1/fixtures/invalid/protocol-version-mismatch.json'
import unknownField from '../../../schemas/neuro-link/v1/fixtures/invalid/unknown-field.json'
import errorResponse from '../../../schemas/neuro-link/v1/fixtures/valid/error-response.json'
import eventAck from '../../../schemas/neuro-link/v1/fixtures/valid/event-ack.json'
import protocolHello from '../../../schemas/neuro-link/v1/fixtures/valid/protocol-hello.json'
import serviceList from '../../../schemas/neuro-link/v1/fixtures/valid/service-list.json'
import sessionOpen from '../../../schemas/neuro-link/v1/fixtures/valid/session-open.json'
import stateResume from '../../../schemas/neuro-link/v1/fixtures/valid/state-resume.json'
import streamNotification from '../../../schemas/neuro-link/v1/fixtures/valid/stream-notification.json'
import turnCancel from '../../../schemas/neuro-link/v1/fixtures/valid/turn-cancel.json'
import turnStart from '../../../schemas/neuro-link/v1/fixtures/valid/turn-start-all-content.json'
import type { ProtocolHelloParams, RpcRequestV1 } from './neuro-link-v1'

const ajv = new Ajv2020({ allErrors: true, strict: false })
const validate = ajv.compile(protocolSchema)

describe('Neuro-Link v1 shared protocol TCK', () => {
  it.each([
    ['protocol-hello', protocolHello],
    ['service-list', serviceList],
    ['session-open', sessionOpen],
    ['turn-cancel', turnCancel],
    ['event-ack', eventAck],
    ['state-resume', stateResume],
    ['turn-start-all-content', turnStart],
    ['stream-notification', streamNotification],
    ['error-response', errorResponse],
  ])('%s is accepted by the canonical schema', (_name, fixture) => {
    expect(validate(fixture), JSON.stringify(validate.errors)).toBe(true)
  })

  it.each([
    ['unknown-field', unknownField],
    ['protocol-version-mismatch', protocolVersionMismatch],
    ['legacy-pipe', legacyPipe],
    ['forged-identity', forgedIdentity],
    ['bad-extension', badExtension],
    ['malformed-content', malformedContent],
  ])('%s is rejected by the canonical schema', (_name, fixture) => {
    expect(validate(fixture)).toBe(false)
  })

  it('keeps the typed hello shape aligned with the fixture', () => {
    const request = protocolHello as unknown as RpcRequestV1<ProtocolHelloParams>
    expect(request.jsonrpc).toBe('2.0')
    expect(request.method).toBe('protocol/hello')
    expect(request.params.protocol).toBe('neuro-link/v1')
    expect(request.params.schema_version).toBe(1)
    expect(request.params.frontend_instance_id).toBe('frontend-1')
  })
})
