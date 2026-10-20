{{- define "lab-apps.syncPolicy" -}}
automated: {prune: true, selfHeal: true}
syncOptions: [CreateNamespace=true, ServerSideApply=true]
# The first sync of an app can come before the CRDs its operator installs: retry until then.
retry:
  limit: 20
  backoff: {duration: 15s, factor: 2, maxDuration: 5m}
{{- end -}}
