import type { VersionInfo } from '../services/types.generated'
import type { MusicTrack } from '../services/backend'

export interface PruneVersionResult {
  nextVersion: VersionInfo | null
  notice?: string
}

/**
 * 版本修剪决策（纯函数）：
 * - 当前选中版本仍在列表中：返回更新后的版本元数据；
 * - 当前选中版本在磁盘被删除：若列表非空顺延至第 0 项并给出通知；若列表为空置 null 并给出通知；
 * - 当前无选中版本：若列表非空默认选中第 0 项。
 */
export function pruneSelectedVersion(
  current: VersionInfo | null,
  availableList: VersionInfo[],
): PruneVersionResult {
  if (!current) {
    return {
      nextVersion: availableList.length > 0 ? availableList[0] : null,
    }
  }

  const existing = availableList.find((v) => v.id === current.id)
  if (existing) {
    return { nextVersion: existing }
  }

  const removedName = current.name || current.id
  if (availableList.length > 0) {
    return {
      nextVersion: availableList[0],
      notice: `已检测到版本【${removedName}】在磁盘被删除，已自动切换至【${availableList[0].name || availableList[0].id}】`,
    }
  }

  return {
    nextVersion: null,
    notice: `已检测到版本【${removedName}】在磁盘被删除，当前暂无可启动版本`,
  }
}

/**
 * 聚焦对账节流决策（纯函数）：
 * 距离上次对账时间未满 throttleMs (默认 2000ms) 时阻断，避免高频切窗口频繁 IO
 */
export function shouldTriggerFocusReconcile(
  now: number,
  lastReconcileTime: number,
  throttleMs = 2000,
): boolean {
  return now - lastReconcileTime >= throttleMs
}

/**
 * 音乐列表失效过滤（纯函数）：
 * 剔除磁盘已不存在的音乐文件
 */
export function filterExistingTracks(
  tracks: MusicTrack[],
  invalidPath: string,
): MusicTrack[] {
  return tracks.filter((t) => t.path !== invalidPath)
}
