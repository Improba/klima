import type { Viewer } from 'cesium'
import type { SimulationResult } from 'src/types'

export function useExport() {
  function exportScreenshot(viewer: Viewer): Promise<Blob> {
    return new Promise((resolve, reject) => {
      viewer.render()
      viewer.canvas.toBlob((blob) => {
        if (blob) {
          resolve(blob)
        } else {
          reject(new Error('Failed to capture canvas'))
        }
      }, 'image/png')
    })
  }

  function exportCSV(result: SimulationResult): string {
    let csv = 'lon,lat,alt,temperature\n'
    for (const t of result.surface_temperatures) {
      csv += `${t.lon},${t.lat},${t.alt},${t.temperature}\n`
    }
    return csv
  }

  function downloadBlob(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  function downloadText(text: string, filename: string, mime = 'text/plain') {
    const blob = new Blob([text], { type: mime })
    downloadBlob(blob, filename)
  }

  return {
    exportScreenshot,
    exportCSV,
    downloadBlob,
    downloadText,
  }
}
