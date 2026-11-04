import { describe, expect, it } from 'vitest'
import {
  pruneSelectedVersion,
  shouldTriggerFocusReconcile,
  filterExistingTracks,
} from '../../src/utils/coreLogic'
import type { VersionInfo } from '../../src/services/types.generated'
import type { MusicTrack } from '../../src/services/backend'

describe('coreLogic 核心逻辑纯函数测试', () => {
  const mockVersions: VersionInfo[] = [
    {
      id: '1.21.1',
      name: '1.21.1',
      version_type: 'release',
      loader: null,
      release_time: '2024-08-08',
      main_class: 'net.minecraft.client.main.Main',
      has_indie: false,
      path: '/game/versions/1.21.1',
      java_version: 21,
    },
    {
      id: '1.20.4-fabric',
      name: '1.20.4-Fabric',
      version_type: 'modded',
      loader: 'Fabric',
      release_time: '2023-12-07',
      main_class: 'net.fabricmc.loader.impl.launch.knot.KnotClient',
      has_indie: true,
      path: '/game/versions/1.20.4-fabric',
      java_version: 17,
    },
  ]

  describe('1. 版本修剪 (pruneSelectedVersion)', () => {
    it('当前选中版本仍在列表中时保持选中，并更新最新元数据', () => {
      const current = { ...mockVersions[0], name: 'OldName' }
      const res = pruneSelectedVersion(current, mockVersions)
      expect(res.nextVersion).toEqual(mockVersions[0])
      expect(res.notice).toBeUndefined()
    })

    it('当前选中的版本被外部删除时，若列表非空顺延至第 0 个版本并给出提示', () => {
      const deletedVersion: VersionInfo = {
        id: '1.19.4-deleted',
        name: 'DeletedVersion',
        version_type: 'release',
        loader: null,
        release_time: null,
        main_class: null,
        has_indie: false,
        path: '/game/versions/1.19.4-deleted',
        java_version: 17,
      }

      const res = pruneSelectedVersion(deletedVersion, mockVersions)
      expect(res.nextVersion).toEqual(mockVersions[0])
      expect(res.notice).toContain('在磁盘被删除')
      expect(res.notice).toContain('已自动切换至【1.21.1】')
    })

    it('当前选中版本被删且列表为空时，置为 null 并给出暂无可启动版本通知', () => {
      const deletedVersion = mockVersions[0]
      const res = pruneSelectedVersion(deletedVersion, [])
      expect(res.nextVersion).toBeNull()
      expect(res.notice).toContain('当前暂无可启动版本')
    })

    it('初始无选中版本时，若列表非空默认选中第 0 项且无报警通知', () => {
      const res = pruneSelectedVersion(null, mockVersions)
      expect(res.nextVersion).toEqual(mockVersions[0])
      expect(res.notice).toBeUndefined()
    })
  })

  describe('2. 聚焦对账节流 (shouldTriggerFocusReconcile)', () => {
    it('2 秒内重复触发应被阻断', () => {
      const last = 10000
      expect(shouldTriggerFocusReconcile(10500, last)).toBe(false)
      expect(shouldTriggerFocusReconcile(11999, last)).toBe(false)
    })

    it('满 2 秒或跨度更长时应允许执行对账', () => {
      const last = 10000
      expect(shouldTriggerFocusReconcile(12000, last)).toBe(true)
      expect(shouldTriggerFocusReconcile(15000, last)).toBe(true)
    })
  })

  describe('3. 音乐列表存在性过滤 (filterExistingTracks)', () => {
    const tracks: MusicTrack[] = [
      { name: 'C418 - Subwoofer Lullaby', file_name: 'subwoofer.mp3', path: '/music/subwoofer.mp3', folder_path: '/music', size_mb: 3.2 },
      { name: 'Lena Raine - Pigstep', file_name: 'pigstep.mp3', path: '/music/pigstep.mp3', folder_path: '/music', size_mb: 4.5 },
      { name: 'C418 - Sweden', file_name: 'sweden.mp3', path: '/music/sweden.mp3', folder_path: '/music', size_mb: 3.8 },
    ]

    it('过滤掉指定失效的音乐路径，保留其他曲目', () => {
      const filtered = filterExistingTracks(tracks, '/music/pigstep.mp3')
      expect(filtered).toHaveLength(2)
      expect(filtered.map((t) => t.path)).toEqual([
        '/music/subwoofer.mp3',
        '/music/sweden.mp3',
      ])
    })

    it('若传入不存在的路径，列表保持不变', () => {
      const filtered = filterExistingTracks(tracks, '/music/not_found.mp3')
      expect(filtered).toHaveLength(3)
    })
  })
})
