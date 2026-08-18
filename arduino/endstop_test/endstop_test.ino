const int XhallSensorPin = 9;   // Сигнальный пин (D9)
const int YhallSensorPin = 10;   // Сигнальный пин (D10)
const int ZhallSensorPin = 11;   // Сигнальный пин (D11)

void setup() {
  Serial.begin(9600);
  
  // Включаем внутренний подтягивающий резистор.
  pinMode(XhallSensorPin, INPUT_PULLUP);
  pinMode(YhallSensorPin, INPUT_PULLUP);
  pinMode(ZhallSensorPin, INPUT_PULLUP);
  
  
  Serial.println("Тест датчика Холла KY-003 запущен...");
  Serial.println("Поднесите магнит к датчику.");
}

int XlastSensorState = 99999999;
int YlastSensorState = 99999999;
int ZlastSensorState = 99999999;

void checkSensor(int pin, int& lastState, const char* name)
{
    int sensorState = digitalRead(pin);
    if (sensorState != lastState) {
      lastState = sensorState;
      if (sensorState == LOW) {
        Serial.print(name);
        Serial.println(": >>> Магнит обнаружен! <<<");
      } else {
        Serial.print(name);
        Serial.println(": Магнитное поле отсутствует");
      }
    }
}

void loop() {
  checkSensor(XhallSensorPin, XlastSensorState, "X");
  checkSensor(YhallSensorPin, YlastSensorState, "Y");
  checkSensor(ZhallSensorPin, ZlastSensorState, "Z");
  //Serial.print(".");
  delay(100);
}