alter type webhook_event_type add value if not exists 'invoice.created' before 'invoice.paid';
