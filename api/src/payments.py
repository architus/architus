import json

import stripe

from src.util import CustomResource
from lib.config import stripe_secret_key


stripe.api_key = stripe_secret_key

intent = stripe.PaymentIntent.create(
    amount=1099,
    currency='usd',
    # Verify your integration in this guide by including this parameter
    metadata={'integration_check': 'accept_a_payment'},
)


class Checkout(CustomResource):
    def post(self):
        session = stripe.checkout.Session.create(
            payment_method_types=['card'],
            line_items=[{
                'price': 'price_1IqqGiHUEjNeJTDOoSGqvp28',
                'quantity': 1,
            }],
            mode='payment',
            success_url='https://example.com/success?session_id={CHECKOUT_SESSION_ID}',
            cancel_url='https://example.com/cancel',
        )
        return json.dumps({'id': session.id})
