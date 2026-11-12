import { Player } from '@/audio/player'
import { Recorder } from '@/audio/recorder'
import { TalkieSocket } from './socket'

// Insted of creating it on component level, we do modules so strict mode doesn't create multiple connections or instances
export const socket = new TalkieSocket()
export const recorder = new Recorder()
export const player = new Player()
