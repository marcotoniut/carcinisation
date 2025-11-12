import * as ToastPrimitive from "@radix-ui/react-toast"
import { useEffect, useState } from "react"
import * as styles from "./Toast.css"

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
      {toasts.map((toast) => {
        const toastTypeClass =
          toast.type === "error"
            ? styles.toastError
            : toast.type === "success"
              ? styles.toastSuccess
              : styles.toastInfo

        return (
          <ToastPrimitive.Root
            key={toast.id}
            className={`${styles.toast} ${toastTypeClass}`}
            duration={toast.type === "error" ? 5000 : 3000}
            onOpenChange={(open) => {
              if (!open) removeToast(toast.id)
            }}
          >
            <div className={styles.toastContent}>
              <ToastPrimitive.Title className={styles.toastTitle}>
                {toast.title}
              </ToastPrimitive.Title>
              {toast.description && (
                <ToastPrimitive.Description className={styles.toastDescription}>
                  {toast.description}
                </ToastPrimitive.Description>
              )}
            </div>
            <ToastPrimitive.Close
              className={styles.toastClose}
              aria-label="Close"
            >
              ×
            </ToastPrimitive.Close>
          </ToastPrimitive.Root>
        )
      })}
      <ToastPrimitive.Viewport className={styles.toastViewport} />
    </ToastPrimitive.Provider>
  )
}
