import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import App from './App.vue'
import styleSource from './style.css?raw'
import { listItems, passwordConfigured } from './api/filehide'

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }))
vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({ onDragDropEvent: vi.fn().mockResolvedValue(() => {}) }) }))
vi.mock('./api/filehide', () => ({
  listItems: vi.fn().mockResolvedValue([
    {
      id: 1,
      path: 'D:\\迅雷下载\\凡人修仙传.txt',
      item_type: 'FILE',
      original_attributes: 32,
      current_status: 1,
      protection_status: 'LOCKED',
      create_time: '2026-08-04T03:07:13Z',
      update_time: '2026-08-04T03:07:13Z'
    }
  ]),
  hideFile: vi.fn(),
  hideFolder: vi.fn(),
  hideFiles: vi.fn().mockResolvedValue({ succeeded: [], failed: [] }),
  hideFolders: vi.fn().mockResolvedValue({ succeeded: [], failed: [] }),
  hidePaths: vi.fn().mockResolvedValue({ succeeded: [], failed: [] }),
  previewPaths: vi.fn().mockResolvedValue([]),
  healthCheck: vi.fn().mockResolvedValue({ checked: 0, healthy: 0, changed: 0 }),
  restoreItem: vi.fn(),
  passwordConfigured: vi.fn().mockResolvedValue(false),
  verifyPassword: vi.fn(),
  setAccessPassword: vi.fn(),
  clearAccessPassword: vi.fn(),
  startRecoveryScan: vi.fn(),
  cancelRecoveryScan: vi.fn(),
  recoverMarkedItem: vi.fn(),
  getAutoLockMinutes: vi.fn().mockResolvedValue(15),
  setAutoLockMinutes: vi.fn(),
  checkSession: vi.fn().mockResolvedValue(true),
  lockSession: vi.fn()
}))

describe('hidden item list', () => {
  it('renders an active item returned by the backend', async () => {
    const wrapper = mount(App)
    await flushPromises()

    expect(wrapper.text()).toContain('凡人修仙传.txt')
  })

  it('keeps active items visible in history', async () => {
    const wrapper = mount(App)
    await flushPromises()

    await wrapper.get('nav button:nth-child(3)').trigger('click')

    expect(wrapper.text()).toContain('凡人修仙传.txt')
  })

  it('shows the quick-lock warning and marks a locked item as locked', async () => {
    const wrapper = mount(App)
    await flushPromises()

    expect(wrapper.text()).toContain('快速锁定不是加密')
    expect(wrapper.text()).toContain('已锁定')
  })

  it('does not load fonts or other resources from Google', () => {
    expect(styleSource).not.toContain('fonts.googleapis.com')
  })

  it('does not offer restore when a lock was removed outside FileHide', async () => {
    vi.mocked(listItems).mockResolvedValueOnce([
      {
        id: 2,
        path: 'D:\\迅雷下载\\外部解除.txt',
        item_type: 'FILE',
        original_attributes: 32,
        current_status: 1,
        protection_status: 'UNLOCKED_EXTERNALLY',
        create_time: '2026-08-04T03:07:13Z',
        update_time: '2026-08-04T03:07:13Z'
      }
    ])

    const wrapper = mount(App)
    await flushPromises()

    expect(wrapper.find('.restore').exists()).toBe(false)
  })

  it('offers an opt-in recovery scan from Settings', async () => {
    const wrapper = mount(App)
    await flushPromises()

    await wrapper.get('nav button:nth-child(4)').trigger('click')

    expect(wrapper.text()).toContain('找回锁定项目')
  })

  it('shows an unlock gate before loading hidden items when password access is enabled', async () => {
    vi.mocked(passwordConfigured).mockResolvedValueOnce(true)

    const wrapper = mount(App)
    await flushPromises()

    expect(wrapper.text()).toContain('输入访问密码')
    expect(wrapper.text()).not.toContain('凡人修仙传.txt')
  })
})
