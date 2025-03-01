let output_div;

export function say_string(data: string): void {
  if (typeof output_div == 'undefined') {
    output_div = document.getElementById('hq-output');
  }
  output_div.innerHTML += data;
  output_div.innerHTML += '<br>';
}