%dw 2.0
output application/json
var generatedAt = vars.requestTime default "1970-01-01T00:00:00Z"
---
{
  id: payload.orderId,
  status: upper(payload.status default "pending"),
  total: (payload.items default [])
           reduce ((item, acc = 0) -> acc + item.price * (item.quantity default 1)),
  values: (payload.values default []) map (value) -> value * 2,
  normalizedStatus: payload.status match {
    case "confirmed" -> "CONFIRMED",
    case var value -> value default "UNKNOWN"
  },
  city: payload.user?.address?.city default "UNKNOWN",
  reference: (payload.values default [])[0] default null,
  generatedAt: generatedAt
}
