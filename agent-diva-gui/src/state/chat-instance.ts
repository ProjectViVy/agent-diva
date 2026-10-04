/**
 * The single chat/session owner instance (DN-2). Console and
 * observability consumers read this controller — they never attach a
 * second event subscription.
 */
import { vivyClient } from '../api/vivy/instance'
import { VivyChatController } from './vivy-chat'

export const vivyChat = new VivyChatController(vivyClient)
