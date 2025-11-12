import * as ToastPrimitive from "@radix-ui/react-toast"
import { useEffect, useState } from "react"
import "./Toast.css"

export type ToastType = "success" | "error" | "info"

export interface ToastMessage {
  id: string
  title: string
  description?: string
  type: ToastType
}

interface ToastProviderProps {
  children: React.ReactNode
}

let toastCount = 0

function generateId() {
  return `toast-${Date.now()}-${++toastCount}`
}

// Global toast queue
let toastQueue: ToastMessage[] = []
let toastListener: ((toasts: ToastMessage[]) => void) | null = null

export function showToast(
  title: string,
  description?: string,
  type: ToastType = "info",
) {
  const toast: ToastMessage = {
    id: generateId(),
    title,
    description,
    type,
  }
  toastQueue = [...toastQueue, toast]
  toastListener?.(toastQueue)
}

export function ToastProvider({ children }: ToastProviderProps) {
  const [toasts, setToasts] = useState<ToastMessage[]>([])

  useEffect(() => {
    toastListener = setToasts
    return () => {
      toastListener = null
    }
  }, [])

  const removeToast = (id: string) => {
    toastQueue = toastQueue.filter((t) => t.id !== id)
    setToasts(toastQueue)
  }

  return (
    <ToastPrimitive.Provider swipeDirection="right">
      {children}
      {toasts.map((toast) => (
        <ToastPrimitive.Root
          key={toast.id}
          className={`toast toast-${toast.type}`}
          duration={toast.type === "error" ? 5000 : 3000}
          onOpenChange={(open) => {
            if (!open) removeToast(toast.id)
          }}
        >
          <div className="toast-content">
            <ToastPrimitive.Title className="toast-title">
              {toast.title}
            </ToastPrimitive.Title>
            {toast.description && (
              <ToastPrimitive.Description className="toast-description">
                {toast.description}
              </ToastPrimitive.Description>
            )}
          </div>
          <ToastPrimitive.Close className="toast-close" aria-label="Close">
            ×
          </ToastPrimitive.Close>
        </ToastPrimitive.Root>
      ))}
      <ToastPrimitive.Viewport className="toast-viewport" />
    </ToastPrimitive.Provider>
  )
}
