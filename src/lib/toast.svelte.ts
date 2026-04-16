export type ToastType = "info" | "success" | "warning" | "error";

export interface Toast {
  id: string;
  message: string;
  type: ToastType;
  duration: number;
}

class ToastState {
  toasts = $state<Toast[]>([]);

  show(message: string, type: ToastType = "info", duration = 3000) {
    const id = Math.random().toString(36).slice(2);
    this.toasts.push({ id, message, type, duration });
    
    if (duration > 0) {
      setTimeout(() => {
        this.toasts = this.toasts.filter(t => t.id !== id);
      }, duration);
    }
  }

  success(message: string, duration?: number) { this.show(message, "success", duration); }
  error(message: string, duration?: number) { this.show(message, "error", duration); }
  warning(message: string, duration?: number) { this.show(message, "warning", duration); }
  info(message: string, duration?: number) { this.show(message, "info", duration); }

  remove(id: string) {
    this.toasts = this.toasts.filter(t => t.id !== id);
  }
}

export const toast = new ToastState();
