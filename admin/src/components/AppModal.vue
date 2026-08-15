<template>
  <Teleport to="body">
    <div v-if="modelValue" class="modal-overlay" @click.self="emit('close')">
      <div class="modal" :style="{ width }">
        <div class="modal-header">
          <h3>{{ title }}</h3>
          <button class="icon-btn" type="button" @click="emit('close')">
            <X :size="18" />
          </button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
        <div v-if="$slots.footer" class="modal-footer">
          <slot name="footer" />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup>
import { X } from '@lucide/vue'

defineProps({
  modelValue: { type: Boolean, default: false },
  title: { type: String, default: '' },
  width: { type: String, default: '480px' }
})

const emit = defineEmits(['close'])
</script>