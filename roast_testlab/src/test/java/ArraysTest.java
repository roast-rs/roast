import org.junit.Test;

import static org.junit.Assert.assertArrayEquals;

public class ArraysTest {

    @Test
    public void reverseByteArrayTest() {
        byte[] input = new byte[] { 'r', 'o', 'a', 's', 't' };
        byte[] expected = new byte[] { 't', 's', 'a', 'o', 'r' };

        byte[] output = Arrays.reverseByteArr(input);
        assertArrayEquals(expected, output);
    }

}