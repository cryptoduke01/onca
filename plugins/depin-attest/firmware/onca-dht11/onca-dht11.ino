// onca-dht11 — ESP32 firmware for a ZeroClaw DePIN attestation node.
//
// Reads a DHT11 temperature sensor and prints one line per reading over USB
// serial, in the exact format the `depin-attest` serial bridge parses:
//
//     onca:reading s=dht11-a v=23.4 u=C seq=42
//
// The host bridge turns each line into an unsigned Solana attestation, enforcing
// reading bounds and the monotonic replay guard. Two deliberate choices here:
//
//   * No wall clock. The ESP32 has no RTC, so this firmware does NOT stamp a
//     timestamp; the host stamps `t` when it ingests the line. One less thing to
//     get wrong (no NTP, no wifi needed just to report a number).
//
//   * The sequence number is persisted in NVS (flash), so it keeps climbing
//     across power cycles and resets. That is what makes the replay guard real:
//     a reboot can never rewind the sequence and let an old reading look fresh.
//
// Board:  ESP32 Dev Module        Wiring (DHT11 module, USB-corner pins):
// Baud:   115200                    +  / VCC  -> 3V3   (bottom-right corner pin)
//                                   OUT / S   -> D15   (GPIO15, 3rd pin up)
//                                   -  / GND  -> GND   (2nd pin up)

#include <DHT.h>
#include <Preferences.h>

#define DHT_PIN 15       // GPIO15 (labelled D15) — the data wire
#define DHT_TYPE DHT11
#define SENSOR_ID "dht11-a"
#define READ_INTERVAL_MS 3000  // DHT11 needs ~2s between reads

DHT dht(DHT_PIN, DHT_TYPE);
Preferences prefs;
unsigned long seq = 0;

void setup() {
  Serial.begin(115200);
  delay(200);
  dht.begin();

  // Load the last sequence from flash and continue from there. First ever boot
  // starts at 0, so the first reading is seq 1.
  prefs.begin("onca", false);
  seq = prefs.getULong("seq", 0);

  Serial.println("# onca dht11 node online");
}

void loop() {
  float c = dht.readTemperature();  // Celsius

  if (isnan(c)) {
    // Instrumented: elapsed ms exposes whether the 3s delay runs (real read)
    // or the board is resetting/spinning (elapsed near zero each line).
    Serial.printf("# read fail  pin=%d  t=%lums\n", DHT_PIN, millis());
  } else {
    seq += 1;
    prefs.putULong("seq", seq);  // persist before emitting, so a crash can't reuse a seq
    Serial.printf("onca:reading s=%s v=%.1f u=C seq=%lu\n", SENSOR_ID, c, seq);
  }

  delay(READ_INTERVAL_MS);
}
